use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*, poly::Rotation};
use std::{marker::PhantomData, process::ChildStderr};

#[derive(Debug, Clone)]
struct FibonacciConfig {
    advice: Column<Advice>,
    selector: Selector,
    instance: Column<Instance>,
}

#[derive(Debug, Clone)]
struct FibonacciChip<F: FieldExt> {
    config: FibonacciConfig,
    _maker: PhantomData<F>,
}

impl<F: FieldExt> FibonacciChip<F> {
    pub fn constract(config: FibonacciConfig) -> Self {
        Self {
            config,
            _maker: PhantomData,
        }
    }
    // custome gateを書いていきます。
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: Column<Advice>,
        instance: Column<Instance>,
    )-> FibonacciConfig {
        let selector = meta.selector();
        // permutationに含める
        meta.enable_equality(advice);
        meta.enable_equality(instance);

        meta.create_gate("add", |meta| {
            // advice | selector
            //    a        s
            //    b 
            //    c

            // この例のミソはここです。Rotationでcellを前後とか何番目かを指定できる
            let s = meta.query_selector(selector);
            let a = meta.query_advice(advice, Rotation::cur());
            let b = meta.query_advice(advice, Rotation::next());
            // nextは(1)と同じです！なるへそ
            // let b = meta.query_advice(advice, Rotation(1));
            let c = meta.query_advice(advice, Rotation(2));
            vec![s * (a + b - c)]
        });

        FibonacciConfig {
            advice,
            selector,
            instance,
        }
    }
    // cellのassignを書いていきます。
    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        nrows: usize,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "entire fibonacci table",
            |mut region| {
                // 2行分のselectorを有効化しています
                // add gateを有効化しているということです
                self.config.selector.enable(&mut region, 0)?;
                self.config.selector.enable(&mut region, 1)?;
            
                let mut a_cell = region.assign_advice_from_instance(
                    || "1", // instanceにある1を持ってくる
                    self.config.instance, // instance列からデス
                    0, // 0行目からです
                    self.config.advice, // advice columnからです
                    0, // adviceの0行目です。
                )?;

                let mut b_cell = region.assign_advice_from_instance(
                    || "1",
                    self.config.instance,
                    1,
                    self.config.advice,
                    1, // adviceの1行目です。
                )?;

                for row in 2..nrows {
                    if row < nrows - 2 {
                        self.config.selector.enable(&mut region, row)?;
                    }

                    let c_cell = region.assign_advice(
                        || "advice",
                        self.config.advice,
                        row,
                        || a_cell.value().copied() + b_cell.value(),
                    )?;
                    // permutation argumentです
                    // 値を更新していく感じ
                    a_cell = b_cell;
                    b_cell = c_cell;
                }

                Ok(b_cell)
            },
        )
    }

    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        cell: AssignedCell<F, F>,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, row)
    }
}

#[derive(Default)]
struct MyCircuit<F>(PhantomData<F>);

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FibonacciConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = meta.advice_column();
        let instance = meta.instance_column();
        FibonacciChip::configure(meta, advice, instance)
    }

    fn synthesize(
        &self, 
        config: Self::Config, 
        mut layouter: impl Layouter<F>
    ) -> Result<(), Error> {
        let chip = FibonacciChip::constract(config);

        let out_cell = chip.assign(layouter.namespace(|| "entire table"), 10)?;
        // 計算結果をexposeでinstance columnに移動する
        chip.expose_public(layouter.namespace(|| "out"), out_cell, 2)?;

        Ok(())
    }
}

#[test]
fn e3_fibonacci_ex2() {
    use std::marker::PhantomData;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    let k = 4;
    let a = Fp::from(1); //F[0]
    let b = Fp::from(1); //F[1]
    let out = Fp::from(55); //F[9]

    let circuit = MyCircuit(PhantomData);

    let public_input = vec![a, b, out];

    let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
    prover.assert_satisfied() 
}