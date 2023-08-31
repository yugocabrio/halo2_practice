
use std::marker::PhantomData;
use halo2_proofs::circuit::Value;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Cell, Chip, Layouter, SimpleFloorPlanner},
    plonk::{Advice, Assigned, Circuit, Column, ConstraintSystem, Error, Fixed, Instance},
    poly::Rotation,
};

#[allow(non_snake_case, dead_code)]
#[derive(Debug, Clone)]
// tableのcolumnを宣言
// Avdice means private input
// Fixed means circuit hardcoded constants (Selector values, constants, lookup table)
// Instance means public input
// 多分この時点ではただの構造体
struct TutorialConfig {
    l: Column<Advice>,
    r: Column<Advice>,
    o: Column<Advice>,

    sl: Column<Fixed>,
    sr: Column<Fixed>,
    so: Column<Fixed>,
    sm: Column<Fixed>,
    sc: Column<Fixed>,
    PI: Column<Instance>,
}

// 上のcolumnを宣言するconfigを marker: PhantomData structと一緒にラップして
// _Chipというstructを作ります。
// Phantom dataというのは、ジェネリクス<F>を埋めるダミー(マーカー)みたいなもの
struct TutorialChip<F: FieldExt> {
    config: TutorialConfig,
    marker: PhantomData<F>,
}

// new function should take in a config and produce a chip.
// new functionでは、回路を表現した行列（TutorialConfig）を引数として受け取り、
// TutorialChipのインスタンスを生成する。（コンストラクタ）
// ここの返り値（インスタンス）は、synthesize functionのlet cs = TutorialChip::new(config)で生成されて、
// csという変数に格納され、TutorailChipのインスタンスに対する参照になる。
// csを介して、TutorialChipのメソッドにアクセスすることができる。
impl<F: FieldExt> TutorialChip<F> {
    fn new(config: TutorialConfig) -> Self {
        TutorialChip {
            config,
            marker: PhantomData,
        }
    }
}

// TutorialChipという型に対してChipという特性（トレイト）
// イマイチ何かよくわかっていない。
impl<F: FieldExt> Chip<F> for TutorialChip<F> {
    type Config = TutorialConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }
 
    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

// TutorialChipにfunctionを持たせる(Columnをどのように組み合わせるかをコードを記述する)
// 計算（加算、乗算）や操作（ワイヤのコピー、Public inputの公開）を回路に行うためのAPIみたいなもの。
// TutorialChipにTutorialComposer traitを通じて、functionの定義を行う。
// このtraitは一連の関数などを定義するところで、具体的な処理は実装されない。次のimplで関数の処理を書く。
trait TutorialComposer<F: FieldExt> {
    fn raw_multiply<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>;

    fn raw_add<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>;
    
    // Ensure two wire value are the same, in effect connecting the wires to each other
    // copy constraintsのチェック（PlonKの場合はPermutation Argument）
    fn copy(&self,
        layouter: &mut impl Layouter<F>,
        a: Cell,
        b: Cell, 
    ) -> Result<(), Error>;

    // Expose a number as a public input to the ciruict
    fn expose_public(
        &self,
        layouter: &mut impl Layouter<F>,
        cell: Cell,
        row: usize,
    ) -> Result<(), Error>;
} 

// layouterは、Region structやRegionLayouter traitと連携して、
// 必要なゲート（operation）を定義することができます。
// assign_regionは、assign_regionのoutputを出力するためのもの
// witness値を割り当てる時は、region structに実装されたassign_advice(Region, annotation, column, offset, to)
// そしてこの関数は、RegionLayouter traitのassign_advice(RegionLayouter, annotation, column, offset, to)を呼び出す。
impl<F: FieldExt> TutorialComposer<F> for TutorialChip<F> {
    fn raw_multiply<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        mut f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>,
    {      
        // layouterとregionの違いがよくわかっていません。。。。
        // layouter traitのassign_regionという関数
        // assign_advice(Region, annotation, column, offset, to)

        // 追記
        // layouterはregionの配置を効率よくやってくれる機能なので、ここでは、Regionを構築して、どのようにcolumnの値を割り当てるかを書く
        layouter.assign_region( // layouterにregionを割り当てる
            || "mul", // エラーメッセージの提供?
            |mut region| { // このクロージャーは、mut regionを引数にとり、それを使用してセルのアサインを行う。
                let mut values = None;
                let lhs = region.assign_advice(
                    //アドバイスコラムに値をアサイン
                    || "lhs", // エラーメッセージ、annotation
                    self.config.l, // advice columnの指定
                    0, // 行のオフセット、あなたが割り当てを始めたい行の相対位置を指定します。
                    || { // toの引数
                        values = Some(f());
                        values.unwrap().map(|v| v.0)
                    },
                )?;
                let rhs = region.assign_advice(
                    || "rhs",
                    self.config.r,
                    0,
                    || values.unwrap().map(|v| v.1),
                )?;

                let out = region.assign_advice(
                    || "out",
                    self.config.o,
                    0,
                    || values.unwrap().map(|v| v.2),
                )?; 
                // これはTutorialCinfigからわかるように、fiexdだけど中身としてはselectorのようなもの
                // l*sl + r*sr + (l*r)*sm - o*so + sc + PI = 0
                // mulだから、smとsoは1にしておくことで、制約がかかるのだよ
                region.assign_fixed(|| "m", self.config.sm, 0, || Value::known(F::one()))?;
                region.assign_fixed(|| "o", self.config.so, 0, || Value::known(F::one()))?;

                Ok((lhs.cell(), rhs.cell(), out.cell()))
            },
        )
    }

    fn raw_add<FM>( // raw_multiplyと同じ！！
        &self,
        layouter: &mut impl Layouter<F>,
        mut f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>,
    {
        layouter.assign_region(
            || "mul",
            |mut region| {
                let mut values = None;
                let lhs = region.assign_advice(
                    || "lhs",
                    self.config.l,
                    0,
                    || {
                        values = Some(f());
                        values.unwrap().map(|v| v.0)
                    },
                )?;
                let rhs = region.assign_advice(
                    || "rhs",
                    self.config.r,
                    0,
                    || values.unwrap().map(|v| v.1),
                )?;

                let out = region.assign_advice(
                    || "out",
                    self.config.o,
                    0,
                    || values.unwrap().map(|v| v.2),
                )?;

                region.assign_fixed(|| "l", self.config.sl, 0, || Value::known(F::one()))?;
                region.assign_fixed(|| "r", self.config.sr, 0, || Value::known(F::one()))?;
                region.assign_fixed(|| "o", self.config.so, 0, || Value::known(F::one()))?;

                Ok((lhs.cell(), rhs.cell(), out.cell()))
            },
        )
    }

    fn copy(&self, layouter: &mut impl Layouter<F>, left: Cell, right: Cell) -> Result<(), Error> {
        layouter.assign_region(
            || "copy",
            |mut region| {
                region.constrain_equal(left, right)?;
                region.constrain_equal(left, right)
            },
        )
    }

    // 一部のセルの値がプルーフ検証時に外部からアクセス可能である必要がある場合に使用されます。
    // 簡単にいうと、とあるセルの数値を、PIのcolumnに移行しますよというもの。
    // MockProverでは、Publicinputで整合性をとれているかのチェックをしていると思われる。
    // cell: 公開領域にエクスポートするセルを指定します。
    // row: 公開領域での cell の行位置を指定します。
    fn expose_public(
        &self,
        layouter: &mut impl Layouter<F>,
        cell: Cell,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell, self.config.PI, row)
    }
}

#[derive(Default)]
struct TutorialCircuit<F: FieldExt> {
    x: Value<F>,
    y: Value<F>,
    constant: F,
}

impl<F: FieldExt> Circuit<F> for TutorialCircuit<F> {
    type Config = TutorialConfig; // CircuitのColumnの定義のこと
    type FloorPlanner = SimpleFloorPlanner; // これはHalo2独自の用語らしい

    // 値が割り当てられていない場合の状態を指定する
    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    // ここでは、custome gateを定義するのが目的
    // 初めにtableのcolumnの役割を書いたTutorialConfigがConfigって宣言されてて、
    // ここのconfigure関数で実際の回路の構成が格納される
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        // adviceなのでprivate inputとwitnessのこと
        let l = meta.advice_column();
        let r = meta.advice_column();
        let o = meta.advice_column();

        // enable_equalityはPermutationの対象に含めるということ
        meta.enable_equality(l);
        meta.enable_equality(r);
        meta.enable_equality(o);

        // fixedなのでconstants、selectorやlookup tableのこと
        // 本当はselector columnがselectorだけど、これはfixedをselectorとして扱っている
        let sm = meta.fixed_column();
        let sl = meta.fixed_column();
        let sr = meta.fixed_column();
        let so = meta.fixed_column();
        let sc = meta.fixed_column();

        #[allow(non_snake_case)]
        // instanceなのでPublic inputのこと
        let PI = meta.instance_column();
        // enable_equalityはPermutationの対象に含めるということ
        meta.enable_equality(PI);

        // ここでcustome gateを定義しています。
        meta.create_gate("mini plonk", |meta| {
            // queryなのでそのcolumnからクエリをします。
            // Rotationというのは、ずらすって意味で、今回cur（current）だから、ずらさない。
            let l = meta.query_advice(l, Rotation::cur());
            let r = meta.query_advice(r, Rotation::cur());
            let o = meta.query_advice(o, Rotation::cur());

            let sl = meta.query_fixed(sl, Rotation::cur());
            let sr = meta.query_fixed(sr, Rotation::cur());
            let so = meta.query_fixed(so, Rotation::cur());
            let sm = meta.query_fixed(sm, Rotation::cur());
            let sc = meta.query_fixed(sc, Rotation::cur());

            // 最終的にこのような制約ができます。
            vec![l.clone() * sl + r.clone() * sr + l * r * sm + (o * so * (-F::one())) + sc]
        });

        TutorialConfig {
            l,
            r,
            o,
            sl,
            sr,
            so,
            sm,
            sc,
            PI,
        }
    }

    // Permutation argumentとセルの割り当てを同時に行います
    // synthesize will calculate the witness and populate the matrix with the corresponding values.
    // Synthesize function does the computation by filling the cells of gates with initial values,
    // intermediate values and output values and putting all the gates together.
    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        // TutorialChipがここを通じて組み込まれる
        let cs = TutorialChip::new(config);

        // Initialise these values so that we can access them more easily outside the block we actually give them a value in
        // これらの値を初期化して、実際に値を与えるブロックの外でより簡単にアクセスできるようにします。
        let x: Value<Assigned<_>> = self.x.into();
        let y: Value<Assigned<_>> = self.y.into();
        let consty = Assigned::from(self.constant);

        // Create x squared
        // Note that the variables named ai for some i are just place holders, meaning that a0 isn't
        // necessarily the first entry in the column a; though in the code we try to make things clear
        let (a0, b0, c0) = cs.raw_multiply(&mut layouter, || x.map(|x| (x, x, x * x)))?;
        cs.copy(&mut layouter, a0, b0)?;

        // Create y squared
        let (a1, b1, c1) = cs.raw_multiply(&mut layouter, || y.map(|y| (y, y, y * y)))?;
        cs.copy(&mut layouter, a1, b1)?;

        // Create xy squared
        let (a2, b2, c2) = cs.raw_multiply(&mut layouter, || {
            x.zip(y).map(|(x, y)| (x * x, y * y, x * x * y * y))
        })?;
        cs.copy(&mut layouter, c0, a2)?;
        cs.copy(&mut layouter, c1, b2)?;

        // Add the constant
        let (a3, b3, c3) = cs.raw_add(&mut layouter, || {
            x.zip(y)
                .map(|(x, y)| (x * x * y * y, consty, x * x * y * y + consty))
        })?;
        cs.copy(&mut layouter, c2, a3)?;

        // Ensure that the constant in the TutorialCircuit struct is correctly used and that the
        // result of the circuit computation is what is expected. (use expose_public))
        cs.expose_public(&mut layouter, b3, 0)?;
        // layouter.constrain_instance(b3, cs.config.PI, 0)?;
        // Below is another way to expose a public value, this time the output value of the computation
        // (Use constrain_instance)
        // cs.expose_public(&mut layouter, c3, 1)?;
        layouter.constrain_instance(c3, cs.config.PI, 1)?;

        Ok(())
    }
}

#[test]
fn e1_tutorial_practice_test() {
    // use halo2_proofs::dev::MockProver;
    //use halo2_proofs::halo2curves::bn256::Fr as Fp;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    // The number of rows in our circuit cannot exceed 2^k. Since our example
    // circuit is very small, we can pick a very small value here.
    // circuitサイズは2の階上でなければならない。
    let k = 4;

    let constant = Fp::from(7);
    let x = Fp::from(5);
    let y = Fp::from(9);
    let z = Fp::from(25 * 81 + 7);

    // TutorialCircuitから、MockProve用のCircuitを定義
    let circuit: TutorialCircuit<Fp> = TutorialCircuit {
        x: Value::known(x),
        y: Value::known(y),
        constant: constant,
    };

    // MockProverが演算の結果と照らし合わせれ検証する用public_inputs
    let mut public_inputs = vec![constant, z];

    // Given the correct public input, our circuit will verify.
    // やっぱMockProverは、public inputを照らし合わせて整合性を確かめるものみたい。
    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
    assert_eq!(prover.verify(), Ok(()));

    // TODO: This broke when Value was introduced to replace Option. Fix it
    // If we try some other public input, the proof will fail!
    public_inputs[0] += Fp::one();
    // let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
    // assert!(prover.verify().is_err());
}