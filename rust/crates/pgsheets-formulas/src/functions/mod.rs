//! The function library: the registry of worksheet functions, and the aggregates every workbook is
//! likely to use.
//!
//! The C# original keeps the registry in a static dictionary, because .NET has no other way to give
//! `SUM` a place to live. Here it is a value the engine owns: extending the language means handing the
//! engine a library with one more entry, and no two workbooks can see each other's functions.

mod conditional;
mod criteria;
mod date;
mod helpers;
mod info;
mod logical;
mod lookup;
mod math;
mod stats;
mod text;
mod wildcards;

use std::collections::HashMap;

use pgsheets_core::{CellError, CellValue};

pub use helpers::compare_ignore_case;

use crate::ast::{AstNode, FormulaContext};
use helpers::{collect_numbers, visit_values};

/// A worksheet function. Arguments arrive unevaluated, so the implementation controls which of them
/// are evaluated and can enumerate a range instead of evaluating it.
pub type BuiltinFn = fn(&[AstNode], &mut dyn FormulaContext) -> CellValue;

/// The functions an engine resolves calls against.
///
/// Lookup is exact first and case-insensitive second, so the common shape — `=SUM(..)` written in
/// capitals — costs one hash lookup and no allocation.
#[derive(Default)]
pub struct FunctionLibrary {
    functions: HashMap<String, BuiltinFn>,
}

impl FunctionLibrary {
    /// The library every engine starts with: the functions this application ships.
    pub fn standard() -> Self {
        let mut library = Self::empty();

        register_math(&mut library);
        register_statistics(&mut library);
        register_logic(&mut library);
        register_text(&mut library);
        register_lookup(&mut library);

        // Dates
        library.register("YEAR", date::year);
        library.register("MONTH", date::month);
        library.register("DAY", date::day);
        library.register("WEEKDAY", date::week_day);

        // Type tests
        library.register("ISBLANK", info::is_blank);
        library.register("ISNUMBER", info::is_number);
        library.register("ISTEXT", info::is_text);
        library.register("ISLOGICAL", info::is_logical);
        library.register("ISERROR", info::is_error);
        library.register("ISNA", info::is_na);
        library.register("ISEVEN", info::is_even);
        library.register("ISODD", info::is_odd);

        library
    }
}

fn register_math(library: &mut FunctionLibrary) {
    // Numbers
    library.register("SUM", sum);
    library.register("PRODUCT", product);
    library.register("ABS", math::abs);
    library.register("SIGN", math::sign);
    library.register("SQRT", math::sqrt);
    library.register("POWER", math::power);
    library.register("EXP", math::exp);
    library.register("LN", math::ln);
    library.register("LOG", math::log);
    library.register("LOG10", math::log10);
    library.register("PI", math::pi);
    library.register("INT", math::int);
    library.register("TRUNC", math::trunc);
    library.register("MOD", math::modulo);
    library.register("ROUND", math::round);
    library.register("ROUNDUP", math::round_up);
    library.register("ROUNDDOWN", math::round_down);
    library.register("CEILING", math::ceiling);
    library.register("FLOOR", math::floor);
    library.register("CONCAT", math::concat);
    library.register("CONCATENATE", math::concat);
}

fn register_statistics(library: &mut FunctionLibrary) {
    library.register("MIN", min);
    library.register("MAX", max);
    library.register("AVERAGE", average);
    library.register("MEDIAN", stats::median);
    library.register("MODE", stats::mode);
    library.register("COUNT", count);
    library.register("COUNTA", count_a);
    library.register("COUNTBLANK", stats::count_blank);
    library.register("COUNTIF", conditional::count_if);
    library.register("SUMIF", conditional::sum_if);
    library.register("AVERAGEIF", conditional::average_if);
    library.register("LARGE", stats::large);
    library.register("SMALL", stats::small);
    library.register("RANK", stats::rank);
    library.register("STDEV", stats::st_dev);
    library.register("VAR", stats::variance);
    library.register("SUMPRODUCT", stats::sum_product);
}

fn register_logic(library: &mut FunctionLibrary) {
    library.register("IF", math::if_function);
    library.register("IFS", logical::ifs);
    library.register("IFERROR", logical::if_error);
    library.register("IFNA", logical::if_na);
    library.register("AND", logical::and);
    library.register("OR", logical::or);
    library.register("XOR", logical::xor);
    library.register("NOT", logical::not);
    library.register("SWITCH", logical::switch);
    library.register("CHOOSE", lookup::choose);
    library.register("TRUE", logical::true_value);
    library.register("FALSE", logical::false_value);
    library.register("NA", logical::na);
}

fn register_text(library: &mut FunctionLibrary) {
    library.register("TEXTJOIN", text::text_join);
    library.register("LEFT", text::left);
    library.register("RIGHT", text::right);
    library.register("MID", text::mid);
    library.register("LEN", text::len);
    library.register("UPPER", text::upper);
    library.register("LOWER", text::lower);
    library.register("PROPER", text::proper);
    library.register("TRIM", text::trim);
    library.register("REPT", text::rept);
    library.register("SUBSTITUTE", text::substitute);
    library.register("FIND", text::find);
    library.register("EXACT", text::exact);
    library.register("VALUE", text::value);
    library.register("CHAR", text::character);
    library.register("CODE", text::code);
}

fn register_lookup(library: &mut FunctionLibrary) {
    library.register("VLOOKUP", lookup::v_look_up);
    library.register("HLOOKUP", lookup::h_look_up);
    library.register("INDEX", lookup::index);
    library.register("MATCH", lookup::match_value);
}

impl FunctionLibrary {
    /// An empty library, for a host that wants to decide the whole function set itself.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Adds or replaces a function. Emptying the registry of a running engine is not possible, so
    /// this is the extension point rather than a mutation of shared state.
    pub fn register(&mut self, name: &str, function: BuiltinFn) {
        self.functions.insert(name.to_string(), function);
    }

    /// Resolves a name, exactly first and case-insensitively second.
    pub fn get(&self, name: &str) -> Option<BuiltinFn> {
        if let Some(function) = self.functions.get(name) {
            return Some(*function);
        }

        // A formula may write a name in any case, which is why the registry is case-insensitive.
        self.functions
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, function)| *function)
    }

    /// True when the library can resolve a name.
    pub fn is_known(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Every registered name, for tests and for a host that wants to list them.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.functions.keys().map(String::as_str)
    }

    /// How many functions the library holds.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// True when the library holds nothing.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

/// Adds every numeric value it is given; booleans count as 1 and 0, and text is ignored.
fn sum(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    let mut total = 0.0;
    let mut error = None;

    for argument in arguments {
        visit_values(argument, context, &mut |value| match value {
            CellValue::Error(_) => {
                error = Some(value);
                false
            }
            CellValue::Number(number) => {
                total += number;
                true
            }
            CellValue::Boolean(flag) => {
                total += if flag { 1.0 } else { 0.0 };
                true
            }
            _ => true,
        });

        if let Some(failure) = error {
            return failure;
        }
    }

    CellValue::number(total)
}

/// Multiplies every numeric value it is given; text and blanks are ignored.
fn product(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    let numbers = match collect_numbers(arguments, context) {
        Ok(numbers) => numbers,
        Err(error) => return error,
    };

    if numbers.is_empty() {
        return CellValue::number(0.0);
    }

    CellValue::number(numbers.iter().product())
}

fn min(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    extremum(arguments, context, true)
}

fn max(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    extremum(arguments, context, false)
}

/// The smallest or largest number among the arguments; text and blanks are ignored.
fn extremum(arguments: &[AstNode], context: &mut dyn FormulaContext, find_minimum: bool) -> CellValue {
    let numbers = match collect_numbers(arguments, context) {
        Ok(numbers) => numbers,
        Err(error) => return error,
    };

    let best = if find_minimum {
        numbers.iter().copied().reduce(f64::min)
    } else {
        numbers.iter().copied().reduce(f64::max)
    };

    CellValue::number(best.unwrap_or(0.0))
}

fn average(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    let numbers = match collect_numbers(arguments, context) {
        Ok(numbers) => numbers,
        Err(error) => return error,
    };

    if numbers.is_empty() {
        return CellValue::error(CellError::DivideByZero);
    }

    CellValue::number(numbers.iter().sum::<f64>() / numbers.len() as f64)
}

/// How many numbers the arguments contribute.
fn count(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    let mut found = 0.0;

    for argument in arguments {
        visit_values(argument, context, &mut |value| {
            if matches!(value, CellValue::Number(_)) {
                found += 1.0;
            }

            true
        });
    }

    CellValue::number(found)
}

/// How many cells hold anything at all.
fn count_a(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    let mut found = 0.0;

    for argument in arguments {
        visit_values(argument, context, &mut |value| {
            if !value.is_empty() {
                found += 1.0;
            }

            true
        });
    }

    CellValue::number(found)
}
