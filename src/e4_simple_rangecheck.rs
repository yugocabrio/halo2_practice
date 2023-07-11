use std::{marker::{PhantomData, PhantomPinned}, path::Components};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter, Value, layouter, floor_planner::V1},
    plonk::{Advice, Assigned, Column, ConstraintSystem, Constraints, Error, Expression, Selector, Circuit},
    poly::Rotation,
};

// valueがRANGE内にあるかどうかをcheckする
// lookup gateは使わないで制約を書く

//  This helper cheks that the value witnessed in a given cell is witin a given range.
//
//  value   |   q_range_check
// ---------------------------
//    v     |         1
// 

// これは何のstructかわからない
#[derive(Debug, Clone)]
struct RangeConstrained<F:FieldExt, const RANGE: usize>(AssignedCell<Assigned<F>, F>);

// plonkishのtableのcolumnを書く
// 今回は、Config書いてconstruct関数書いて、Chip書いてみたいなくだりは無くて、
// Cofigを起点に書いていく
#[derive(Debug, Clone)]
struct RangeCheckConfig<F: FieldExt, const RANGE: usize> {
    value: Column<Advice>,
    q_range_check: Selector,
    _maker: PhantomData<F>,
}

impl<F: FieldExt, const RANGE: usize> RangeCheckConfig<F, RANGE> {
    // MyCIrcuitからこのconfigureを呼び出す
    // このConfigureでは、Custome gateの定義です
    pub fn configure(meta: &mut ConstraintSystem<F>, value: Column<Advice>) -> Self {
        let q_range_check = meta.selector();
        // range check coustome gateです
        meta.create_gate("range check", |meta|{
            let q = meta.query_selector(q_range_check);
            let value = meta.query_advice(value, Rotation::cur());

            // ⭐️ここのロジックがどうやって実装されているのか未理解
            // given a range R and a value v, returns the expression
            // (v) * (1-v) * (2-v) * (3-v) * ... * (R-1-v)
            let range_check = |range: usize, value: Expression<F>| {
                assert!(range > 0);
                (1..range).fold(value.clone(), |expr, i| {
                    expr * (Expression::Constant(F::from(i as u64)) - value.clone())
                })
            };

            Constraints::with_selector(q, [("range check", range_check(RANGE, value))])
        });

        Self{
            q_range_check,
            value,
            _maker: PhantomData,
        }

    }

    // MyCircuitのsynthesizeから呼び出される
    pub fn assign(
        &self, 
        mut layouter: impl Layouter<F>,
        value: Value<Assigned<F>>,
    ) -> Result<RangeConstrained<F, RANGE>, Error> {
        layouter.assign_region(
            || "Assign value",
            |mut region|{
                let offset = 0;

                // q_range_checkをenableする（有効化）
                self.q_range_check.enable(&mut region, offset)?;

                // Assign value
                // ⭐️ ちょっとここ何しているか考える
                region
                    .assign_advice(|| "value", self.value, offset, || value)
                    .map(RangeConstrained)
            },
        )
    } 
}

#[derive(Default)]
struct Mycircuit<F: FieldExt, const RANGE: usize> {
    value: Value<Assigned<F>>,
}

impl<F: FieldExt, const RANGE: usize> Circuit<F> for Mycircuit<F, RANGE> {
    type Config = RangeCheckConfig<F, RANGE>;
    // シンプルでは無くて、V1とは何なのか？
    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let value = meta.advice_column();
        RangeCheckConfig::configure(meta, value)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        config.assign(layouter.namespace(|| "Assign value"), self.value)?;

        Ok(())
    } 
}

#[test]
fn e4_simple_rangecheck() {
    use halo2_proofs::{dev::MockProver, pasta::Fp};
    let k = 4;
    const RANGE: usize = 8; // 3bit check

    let circuit = Mycircuit::<Fp, RANGE> {
        value: Value::known(Fp::from(4 as u64).into()),
    };

    let prover = MockProver::run(k, &circuit, vec![]).unwrap();
    prover.assert_satisfied()

}
