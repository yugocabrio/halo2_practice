use std::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Layouter, Value},
    plonk::{ConstraintSynstem, Error, TableColumn},
};

// A look up table of values from 0..RANGE
#[derive(Debug, Clone)]
pub(super) struct RangeTableConfig<F: FieldExt, const RANGE: usize> {
    pub(super) value: TableColumn,
    _maker: PhantomData<F>,
}

impl<F: FieldExt, const RANGE: usize> RangeTableConfig<F, RANGE> {
    pub(super) fn configure(meta: &mut ConstraintSynstem<F>) -> Self {
        assert_eq!(1 << NUM_BITS, RANGE);

        let num_bits = meta.lookup_table_column();
        let value = meta.lookup_table_column();

        Self {
            num_bits,
            value,
            _maker: PhantomData,
        }
    }

    pub(super) fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "load range-check table",
            |mut table| {
                let mut offset = 0;

                // Assign (num_bits = 1 , value = 0)
                {
                    table.assign_cell(
                        || "assign num_bits",
                        self.num_bits,
                        offset,
                        || Value::known(F::one()),
                    )?;
                    table.assign_cell(
                        || "assign value",
                        self.value,
                        offset,
                        || Value::known(F::zero()),
                    )?;

                    offset += 1;
                }

                for num_bits in 1..=NUM_BITS {
                    for value in (1<< (num_bits -1)..1 <<  num_bits) {
                        table.assign_cell(
                            || "assign numbits",
                            self.num_bits,
                            offset,
                            || Value::known(F::from(num_bits as u64)),
                        )?;
                        table.assign_cell(
                            || "assign value",
                            self.value,
                            offset,
                            || Value::known(F::from(value as u64)),
                        )?;
                        offset += 1;
                    }
                }

                Ok(())
            },
        )
    }
}