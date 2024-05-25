use std::collections::{BTreeMap, HashMap};

use ts_rs::TS;

/// Since macros cannot lookup other items in the source code, any user types must 'register'
/// themselves into a central repository so that their types can be collated, ensuring that they're
/// only inserted into the generated types once.
pub trait ExportType: TS {
    #[allow(unused_variables)]
    fn export(registry: &mut BTreeMap<String, String>) {}
}

macro_rules! impl_export_type {
    ($($t:ident<$($generic:ident),*>),*) => {
        $(impl<$($generic),*> ExportType for $t<$($generic),*>
            where $($generic: ts_rs::TS + crate::ExportType),*
            {
            fn export(register: &mut BTreeMap<String, String>) {
                $(impl_export_type!(generic: $generic, register);)*
            }
        })*
    };

    ($($t:ty),*) => {
        $(impl ExportType for $t {})*
    };

    (tuple: $t:ident) => {
        impl<$t> ExportType for ($t,)
            where $t: ts_rs::TS + crate::ExportType,
        {
            fn export(register: &mut BTreeMap<String, String>) {
                impl_export_type!(generic: $t, register);
            }
        }
    };

    (tuple: $t:ident $(, $t_other:ident)*) => {
        impl<$t, $($t_other),*> ExportType for ($t, $($t_other),*)
            where $t: ts_rs::TS + crate::ExportType,
            $($t_other: ts_rs::TS + crate::ExportType),*
        {
            fn export(register: &mut BTreeMap<String, String>) {
                impl_export_type!(generic: $t, register);
                $(impl_export_type!(generic: $t_other, register);)*
            }
        }

        impl_export_type!(tuple: $($t_other),*);
    };

    (generic: $generic:ident, $register:ident) => {
        <$generic as ExportType>::export($register)
    };
}

impl_export_type!(
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    String,
    &'static str,
    bool,
    char,
    ()
);
impl_export_type!(
    Vec<T>,
    Box<T>,
    Option<T>,
    Result<T, E>,
    HashMap<K, V>
);
impl_export_type!(tuple: T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
