use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter, Value, floor_planner::V1},
    plonk::{Advice, Assigned, Column, ConstraintSystem, Constraints, Error, Expression, Selector, Circuit},
    poly::Rotation,
};
use std::marker::PhantomData;

mod e5_lookup_table;
use e5_lookup_table::*;

// This helper checks that the value witnessed in a given cell is within a given range.
// Depending on the range, this helper uses either a range-check expression (for small ranges)
// or a lookup table
//
//        value     |    q_range_check    |   q_lookup  |  table_value  |
//       ----------------------------------------------------------------
//          v_0     |         1           |      0      |       0       |
//          v_1     |         0           |      1      |       1       |

#[derive(Debug, Clone)]
// A range-constrained value in the circuit produced by the RangeCheckConfig.
struct RangeConstrained<F: FieldExt, const RANGE: usize>(AssignedCell<Assigned<F>, F>);

#[derive(Debug, Clone)]
struct RangeCheckConfig<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> {
    q_range_check: Selector,
    q_lookup: Selector,
    value: Column<Advice>,
    table: RangeTableConfig<F, LOOKUP_RANGE>,
}

impl<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> RangeCheckConfig<F, RANGE, LOOKUP_RANGE> {
    pub fn configure(meta: &mut ConstraintSystem<F>, value: Column<Advice>) -> Self {
        let q_range_check = meta.selector();
        let q_lookup = meta.complex_selector(); // research the meaning later
        let table = RangeTableConfig::configure(meta);

        // range check gate without lookup
        meta.create_gate("rangecheck", |meta| {
            //  value   |   q_range_check
            // ---------------------------
            //    v     |         1
            // 

            let q = meta.query_selector(q_range_check);
            let value = meta.query_advice(value, Rotation::cur());

            // Given a range R and a value v, returns the expression
            // (v) * (1-v) * (2-v) * ... * (R - 1 - v)
            let range_check = |range: usize, value: Expression<F>| {
                assert!(range > 0);
                (1..range).fold(value.clone(), |expr, i| {
                    expr * (Expression::Constant(F::from(i as u64)) - value.clone())
                })
            };

            // research later
            Constraints::with_selector(q, [("range check", range_check(RANGE, value))])
        });

        // range check with lookup
        meta.lookup(|meta| {
            let q_lookup = meta.query_selector(q_lookup);
            let value = meta.query_advice(value, Rotation::cur());

            // なんで等式じゃないか考える
            vec![(q_lookup * value, table.value)]
        });

        Self {
            q_range_check,
            q_lookup,
            value,
            table,
        }

    }

    // without lookupのcellの割り当て
    pub fn assign_simple(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<Assigned<F>>,
    ) -> Result<RangeConstrained<F, RANGE>, Error> {
        layouter.assign_region(
            || "Assign value for simple range check",
            |mut region| {
                let offset = 0;

                // Enable q_range_check
                self.q_range_check.enable(&mut region, offset)?;

                // Assign value
                region
                .assign_advice(|| "value", self.value, offset, || value)
                .map(RangeConstrained)
            },
        )
    }

    // lookupのcellの割り当て
    pub fn assign_lookup(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<Assigned<F>>,
    ) -> Result<RangeConstrained<F, LOOKUP_RANGE>, Error> {
        layouter.assign_region(
            || "Assigned value for lookup range check",
            |mut region| {
                let offset = 0;

                // Enable q_lookup
                self.q_lookup.enable(&mut region, offset)?;

                // Assign value
                region
                    .assign_advice(|| "value", self.value, offset, || value)
                    .map(RangeConstrained)
            },
        )
    }

}

#[derive(Default)]
struct MyCircuit<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> {
    value: Value<Assigned<F>>,
    lookup_value: Value<Assigned<F>>,
}

impl<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> Circuit<F> for MyCircuit<F, RANGE, LOOKUP_RANGE> {
    type Config = RangeCheckConfig<F, RANGE, LOOKUP_RANGE>;
    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    // custome gateです
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let value = meta.advice_column();
        RangeCheckConfig::configure(meta, value)        
    }
    // sythesizeとassginって何が違ったけ？
    // permutation（今回はない）とセルの割り当てを行う、lookupもか。
    fn synthesize(
        &self, 
        config: Self::Config, 
        mut layouter: impl Layouter<F>
    ) -> Result<(), Error> {
        config.table.load(&mut layouter)?;

        config.assign_simple(layouter.namespace(|| "Asiign simple value"), self.value)?;
        config.assign_lookup(
            layouter.namespace(|| "Assign lookup value"),
            self.lookup_value,
        )?;

        Ok(())
    }
}

#[test]
fn e5_lookup_rangecheck() {
    use std::marker::PhantomData;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    
}