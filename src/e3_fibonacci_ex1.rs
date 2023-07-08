use std::{marker::PhantomData, process::ChildStderr};
use halo2_proofs::{arithmetic::FieldExt, circuit::{*, self}, plonk::*, poly::Rotation};

#[derive(Debug, Clone)]
// ここでテーブルのcolumnの一覧を書いていきます。
struct FibonacciConfig {
    pub col_a: Column<Advice>,
    pub col_b: Column<Advice>,
    pub col_c: Column<Advice>,
    pub selector: Selector,
    pub instance: Column<Instance>,
}
#[derive(Debug, Clone)]
struct FibonacciChip<F: FieldExt> {
    config: FibonacciConfig,
    _maker: PhantomData<F>
}

impl<F: FieldExt> FibonacciChip<F> {
    pub fn construct(config: FibonacciConfig) -> Self {
        Self {
            config,
            _maker: PhantomData,
        }
    }

    // configureでcustome gateの定義をする
    pub fn configure(meta: &mut ConstraintSystem<F>) -> FibonacciConfig {
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let selector = meta.selector();
        let instance = meta.instance_column();

        // permutation argumentに含めるか否か
        // selectorは要らんかったわw 当たり前体操
        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        // create_gateでクエリするcolumnのcellを指定して、constraintとなる方程式を書きます。
        meta.create_gate("add", |meta|{
            let s = meta.query_selector(selector);
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());
            vec![s * (a + b - c)]
        });

        //　多分この戻り値にconstraintがかかっている
        FibonacciConfig {
            col_a,
            col_b,
            col_c,
            selector,
            instance,
        }
    }

    // 1行目の割り当てを書いていく
    // ここでやっていることは、instance columnの1,2にそれぞれ、1番目のrowにa, bに入れていきます。（理解したわ）
    // それで、c cellにa+bの結果を入れていきます。
    #[allow(clippy::type_complexity)]
    pub fn assign_first_row(
        &self,
        mut layouter: impl Layouter<F>,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        layouter.assign_region(
            || "first row",
            |mut region| {
                // この行でslector列をenableしている
                // この場合は、add gateを有効化している
                // e1, e2でこれがないのは、fiexdをselector列として扱っているからや、理解！
                self.config.selector.enable(&mut region, 0)?;

                let a_cell = region.assign_advice_from_instance(
                    || "f(0)",
                    self.config.instance,
                    0,
                    self.config.col_a,
                    0)?;

                let b_cell = region.assign_advice_from_instance(
                    || "f(1)",
                    self.config.instance,
                    1,
                    self.config.col_b,
                    0)?;

                let c_cell = region.assign_advice(
                    || "a + b",
                    self.config.col_c,
                    0,
                    || a_cell.value().copied() + b_cell.value(),
                )?;

                Ok((a_cell, b_cell, c_cell))
            },
        )
    }

    // 2行目以降の割り当てを書いていく
    pub fn assign_row(
        &self,
        mut layouter: impl Layouter<F>,
        prev_b: &AssignedCell<F, F>,
        prev_c: &AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "next row",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;

                // previousのb,cをコピーして、今の行のa,bにコピーする
                prev_b.copy_advice(
                    || "a",
                    &mut region,
                    self.config.col_a,
                    0,
                )?;
                prev_c.copy_advice(
                    || "b",
                    &mut region,
                    self.config.col_b,
                    0,
                )?;
                // そしてcに足したものを書きます
                let c_cell = region.assign_advice(
                    || "c",
                    self.config.col_c,
                    0,
                    || prev_b.value().copied() + prev_c.value(),
                )?;

                Ok(c_cell)
            },
        )
    }

    // expose_public
    // instance columnにジャンプさせるってことっす！
    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        cell: &AssignedCell<F, F>,
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

    // custome gateでっせ。
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FibonacciChip::configure(meta)
    }

    //  cellの割り当てとpermutation argumentを同時に行うことができます。
    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>
    ) -> Result<(), Error> {
        let chip = FibonacciChip::construct(config);

        // row 0（1番最初のrow）の割り当てを実際に行う
        let (_, mut prev_b, mut prev_c) = chip.assign_first_row(layouter.namespace(|| "first row"))?;
        for _i in 3..10 {
            let c_cell = chip.assign_row(layouter.namespace(|| "next row"), &prev_b, &prev_c)?;
            // ここでpermutation argumentの割り当て
            prev_b = prev_c;
            prev_c = c_cell;
        }
        // instance columnの2 rowに最後のrowの値を移します。
        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 2)?;

        Ok(())
    }

}

#[test]
fn e3_fibonacci_ex1() {
    use halo2_proofs::{dev::MockProver, pasta::Fp};
    // 回路のサイズやんな？ 2^4 = 16行分できる
    let k = 4;
    let a = Fp::from(1);
    let b = Fp::from(1);
    // 1..10でフィボナッチした時の答えっす。
    let out = Fp::from(55);

    let circuit = MyCircuit(PhantomData);

    let public_input = vec![a, b, out];
    // public_inputとtableでできたものが一致するかのMock検証
    let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
    prover.assert_satisfied();
}
// cargo test --features "dev-graph" e3_fibonacci_ex1_plot
#[cfg(feature = "dev-graph")]
#[test]
fn e3_fibonacci_ex1_plot() {
    use halo2_proofs::{pasta::Fp};
    use plotters::prelude::*;

    let root = BitMapBackend::new("e3_fibonacci_ex1_plot.png", (1024, 3096)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let root = root.titled("e3_fibonacci_ex1_plot", ("sans-serif", 60)).unwrap();

    let circuit = MyCircuit::<Fp>(PhantomData);
    halo2_proofs::dev::CircuitLayout::default()
        .render(4, &circuit, &root)
        .unwrap();
}