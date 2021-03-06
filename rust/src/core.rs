use crate::env::Env;
use crate::printer::pr_str;
use crate::reader::read_str;
use crate::types::MalValueType::{
    Atom, False, Keyword, List, MalFunc, Map, Nil, Number, RustFunc, Str, Symbol, True, Vector,
};
use crate::types::{MalError, MalList, MalMap, MalResult, MalValue, MalVector};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::error::Error;
use std::fs;
use std::slice;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn ns(env: &Env) -> Vec<(&'static str, MalValue)> {
    vec![
        ("+", MalValue::new_rust_func(add, env)),
        ("-", MalValue::new_rust_func(subtract, env)),
        ("*", MalValue::new_rust_func(multiply, env)),
        ("/", MalValue::new_rust_func(divide, env)),
        ("prn", MalValue::new_rust_func(prn, env)),
        ("println", MalValue::new_rust_func(mal_println, env)),
        ("pr-str", MalValue::new_rust_func(mal_pr_str, env)),
        ("str", MalValue::new_rust_func(mal_str, env)),
        ("list", MalValue::new_rust_func(list, env)),
        ("list?", MalValue::new_rust_func(is_list, env)),
        ("cons", MalValue::new_rust_func(cons, env)),
        ("concat", MalValue::new_rust_func(concat, env)),
        ("empty?", MalValue::new_rust_func(empty, env)),
        ("count", MalValue::new_rust_func(count, env)),
        ("nth", MalValue::new_rust_func(nth, env)),
        ("first", MalValue::new_rust_func(first, env)),
        ("rest", MalValue::new_rust_func(rest, env)),
        ("conj", MalValue::new_rust_func(conj, env)),
        ("=", MalValue::new_rust_func(equals, env)),
        ("<", MalValue::new_rust_func(lt, env)),
        ("<=", MalValue::new_rust_func(lte, env)),
        (">", MalValue::new_rust_func(gt, env)),
        (">=", MalValue::new_rust_func(gte, env)),
        ("read-string", MalValue::new_rust_func(read_string, env)),
        ("slurp", MalValue::new_rust_func(slurp, env)),
        ("eval", MalValue::new_rust_func(mal_eval, env)),
        ("atom", MalValue::new_rust_func(atom, env)),
        ("atom?", MalValue::new_rust_func(is_atom, env)),
        ("deref", MalValue::new_rust_func(deref_atom, env)),
        ("reset!", MalValue::new_rust_func(reset_atom, env)),
        ("swap!", MalValue::new_rust_func(swap_atom, env)),
        ("throw", MalValue::new_rust_func(throw, env)),
        ("nil?", MalValue::new_rust_func(is_nil, env)),
        ("true?", MalValue::new_rust_func(is_true, env)),
        ("false?", MalValue::new_rust_func(is_false, env)),
        ("symbol?", MalValue::new_rust_func(is_symbol, env)),
        ("symbol", MalValue::new_rust_func(symbol, env)),
        ("keyword?", MalValue::new_rust_func(is_keyword, env)),
        ("keyword", MalValue::new_rust_func(keyword, env)),
        ("apply", MalValue::new_rust_func(apply, env)),
        ("map", MalValue::new_rust_func(map, env)),
        ("vector", MalValue::new_rust_func(vector, env)),
        ("vector?", MalValue::new_rust_func(is_vector, env)),
        ("sequential?", MalValue::new_rust_func(is_sequential, env)),
        ("hash-map", MalValue::new_rust_func(hash_map, env)),
        ("map?", MalValue::new_rust_func(is_map, env)),
        ("assoc", MalValue::new_rust_func(assoc, env)),
        ("dissoc", MalValue::new_rust_func(dissoc, env)),
        ("get", MalValue::new_rust_func(get, env)),
        ("contains?", MalValue::new_rust_func(contains, env)),
        ("keys", MalValue::new_rust_func(keys, env)),
        ("vals", MalValue::new_rust_func(vals, env)),
        ("readline", MalValue::new_rust_func(readline, env)),
        ("meta", MalValue::new_rust_func(meta, env)),
        ("with-meta", MalValue::new_rust_func(with_meta, env)),
        ("string?", MalValue::new_rust_func(is_string, env)),
        ("number?", MalValue::new_rust_func(is_number, env)),
        ("fn?", MalValue::new_rust_func(is_fn, env)),
        ("macro?", MalValue::new_rust_func(is_macro, env)),
        ("time-ms", MalValue::new_rust_func(time_ms, env)),
        ("seq", MalValue::new_rust_func(seq, env)),
    ]
}

static mut EVAL_FUNC: fn(ast: &MalValue, env: &mut Env) -> MalResult = dummy_eval;

fn dummy_eval(_: &MalValue, _: &mut Env) -> MalResult {
    panic!("core EVAL_FUNC was not set. You must call core::set_eval_func().")
}

pub fn set_eval_func(func: fn(ast: &MalValue, env: &mut Env) -> MalResult) {
    unsafe {
        EVAL_FUNC = func;
    }
}

fn core_eval(ast: &MalValue, env: &mut Env) -> MalResult {
    unsafe { EVAL_FUNC(ast, env) }
}

fn core_apply(function: &MalValue, args: &[MalValue], _env: &mut Env) -> MalResult {
    match *function.mal_type {
        RustFunc(ref rust_function) => {
            Ok((rust_function.func)(&args, &mut rust_function.env.clone())?)
        }
        MalFunc(ref mal_func) => {
            let mut func_env =
                Env::with_binds(Some(&mal_func.outer_env), &mal_func.parameters, &args)?;
            core_eval(&mal_func.body, &mut func_env)
        }
        _ => Err(MalError::RustFunction("Expected function.".to_string())),
    }
}

fn arg_count_eq(args: &[MalValue], expected: usize) -> Result<(), MalError> {
    if args.len() != expected {
        return Err(MalError::RustFunction(format!(
            "Expected {} argument{}, got {}",
            expected,
            if expected == 1 { "" } else { "s" },
            args.len()
        )));
    }

    Ok(())
}

fn arg_count_gte(args: &[MalValue], min_args: usize) -> Result<(), MalError> {
    if args.len() < min_args {
        return Err(MalError::RustFunction(format!(
            "Expected at least {} argument{}, got {}",
            min_args,
            if min_args == 1 { "" } else { "s" },
            args.len()
        )));
    }

    Ok(())
}

fn get_number_arg(arg: &MalValue) -> Result<f64, MalError> {
    if let Number(n) = *arg.mal_type {
        Ok(n)
    } else {
        Err(MalError::RustFunction(
            "Argument must be a number".to_string(),
        ))
    }
}

fn add(args: &[MalValue], _env: &mut Env) -> MalResult {
    eval_arithmetic_operation(args, |a, b| a + b)
}

fn subtract(args: &[MalValue], _env: &mut Env) -> MalResult {
    eval_arithmetic_operation(args, |a, b| a - b)
}

fn multiply(args: &[MalValue], _env: &mut Env) -> MalResult {
    eval_arithmetic_operation(args, |a, b| a * b)
}

fn divide(args: &[MalValue], _env: &mut Env) -> MalResult {
    eval_arithmetic_operation(args, |a, b| a / b)
}

fn eval_arithmetic_operation(args: &[MalValue], op: fn(f64, f64) -> f64) -> MalResult {
    arg_count_eq(args, 2)?;

    let arg_1 = get_number_arg(&args[0])?;
    let arg_2 = get_number_arg(&args[1])?;

    Ok(MalValue::new(Number(op(arg_1, arg_2))))
}

fn list(args: &[MalValue], _env: &mut Env) -> MalResult {
    Ok(MalValue::new_list(args.to_vec()))
}

fn is_list(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if args[0].is_list() {
        Ok(MalValue::new(True))
    } else {
        Ok(MalValue::new(False))
    }
}

fn cons(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    match *args[1].mal_type {
        List(MalList { ref vec, .. }) | Vector(MalVector { ref vec, .. }) => {
            let mut new_vec = Vec::with_capacity(vec.len() + 1);
            new_vec.push(args[0].clone());
            new_vec.extend_from_slice(vec);

            Ok(MalValue::new_list(new_vec))
        }
        _ => Err(MalError::RustFunction("Invalid 2nd argument".to_string())),
    }
}

fn concat(args: &[MalValue], _env: &mut Env) -> MalResult {
    let mut reult_vec = Vec::new();

    for arg in args {
        match *arg.mal_type {
            List(MalList { ref vec, .. }) | Vector(MalVector { ref vec, .. }) => {
                reult_vec.extend_from_slice(vec);
            }
            _ => Err(MalError::RustFunction("Invalid argument".to_string()))?,
        }
    }

    Ok(MalValue::new_list(reult_vec))
}

fn empty(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    match *args[0].mal_type {
        List(MalList { ref vec, .. }) | Vector(MalVector { ref vec, .. }) => {
            if vec.is_empty() {
                Ok(MalValue::new(True))
            } else {
                Ok(MalValue::new(False))
            }
        }
        Str(ref s) => {
            if s.is_empty() {
                Ok(MalValue::new(True))
            } else {
                Ok(MalValue::new(False))
            }
        }
        _ => Err(MalError::RustFunction("Invalid argument".to_string())),
    }
}

fn count(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    match *args[0].mal_type {
        List(MalList { ref vec, .. }) | Vector(MalVector { ref vec, .. }) => {
            Ok(MalValue::new(Number(vec.len() as f64)))
        }
        Str(ref s) => Ok(MalValue::new(Number(s.len() as f64))),
        Nil => Ok(MalValue::new(Number(0.))),
        _ => Err(MalError::RustFunction("Invalid argument".to_string())),
    }
}

fn nth(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    let index = get_number_arg(&args[1])?;

    if let List(MalList { ref vec, .. }) | Vector(MalVector { ref vec, .. }) = *args[0].mal_type {
        vec.get(index as usize)
            .cloned()
            .ok_or_else(|| MalError::RustFunction("nth: index out of range".to_string()))
    } else {
        Err(MalError::RustFunction("Invalid argument".to_string()))
    }
}

fn first(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    match *args[0].mal_type {
        List(MalList { ref vec, .. }) | Vector(MalVector { ref vec, .. }) => {
            Ok(vec.get(0).cloned().unwrap_or_else(MalValue::nil))
        }
        Nil => Ok(MalValue::nil()),
        _ => Err(MalError::RustFunction("Invalid argument".to_string())),
    }
}

fn rest(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    match *args[0].mal_type {
        List(MalList { ref vec, .. }) | Vector(MalVector { ref vec, .. }) => {
            Ok(if vec.is_empty() {
                MalValue::new_list(Vec::new())
            } else {
                MalValue::new_list(Vec::from(&vec[1..]))
            })
        }
        Nil => Ok(MalValue::new_list(Vec::new())),
        _ => Err(MalError::RustFunction("Invalid argument".to_string())),
    }
}

fn conj(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_gte(args, 2)?;

    match *args[0].mal_type {
        List(MalList { ref vec, .. }) => {
            let mut new_vec = Vec::with_capacity(vec.len() + args.len() - 1);
            let start_vec: Vec<MalValue> = args[1..].iter().rev().cloned().collect();
            new_vec.extend_from_slice(&start_vec);
            new_vec.extend_from_slice(vec);

            Ok(MalValue::new_list(new_vec))
        }
        Vector(MalVector { ref vec, .. }) => {
            let mut new_vec = Vec::with_capacity(vec.len() + args.len() - 1);
            new_vec.extend_from_slice(vec);
            new_vec.extend_from_slice(&args[1..]);

            Ok(MalValue::new_vector(new_vec))
        }
        _ => Err(MalError::RustFunction("Invalid argument".to_string())),
    }
}

fn equals(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    Ok(MalValue::new_boolean(args[0] == args[1]))
}

fn lt(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    let arg_1 = get_number_arg(&args[0])?;
    let arg_2 = get_number_arg(&args[1])?;

    Ok(MalValue::new_boolean(arg_1 < arg_2))
}

fn lte(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    let arg_1 = get_number_arg(&args[0])?;
    let arg_2 = get_number_arg(&args[1])?;

    Ok(MalValue::new_boolean(arg_1 <= arg_2))
}

fn gt(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    let arg_1 = get_number_arg(&args[0])?;
    let arg_2 = get_number_arg(&args[1])?;

    Ok(MalValue::new_boolean(arg_1 > arg_2))
}

fn gte(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    let arg_1 = get_number_arg(&args[0])?;
    let arg_2 = get_number_arg(&args[1])?;

    Ok(MalValue::new_boolean(arg_1 >= arg_2))
}

fn pr_strs(strs: &[MalValue], print_readably: bool) -> Vec<String> {
    strs.iter().map(|arg| pr_str(arg, print_readably)).collect()
}

fn prn(args: &[MalValue], _env: &mut Env) -> MalResult {
    println!("{}", pr_strs(args, true).join(" "));

    Ok(MalValue::nil())
}

fn mal_println(args: &[MalValue], _env: &mut Env) -> MalResult {
    println!("{}", pr_strs(args, false).join(" "));

    Ok(MalValue::nil())
}

fn mal_pr_str(args: &[MalValue], _env: &mut Env) -> MalResult {
    Ok(MalValue::new(Str(pr_strs(args, true).join(" "))))
}

fn mal_str(args: &[MalValue], _env: &mut Env) -> MalResult {
    Ok(MalValue::new(Str(pr_strs(args, false).join(""))))
}

fn read_string(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Str(ref arg) = *args[0].mal_type {
        read_str(arg)
    } else {
        Err(MalError::RustFunction(
            "read_string expects argument to be of type String".to_string(),
        ))
    }
}

fn slurp(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Str(ref arg) = *args[0].mal_type {
        let file_content = fs::read_to_string(arg)
            .map_err(|e| MalError::RustFunction(format!("slurp: {}", e.description())))?;

        Ok(MalValue::new(Str(file_content)))
    } else {
        Err(MalError::RustFunction(
            "slurp expects argument to be of type String".to_string(),
        ))
    }
}

fn mal_eval(args: &[MalValue], env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    core_eval(&args[0], env)
}

fn atom(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    Ok(MalValue::new_atom(args[0].clone()))
}

fn is_atom(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    Ok(MalValue::new_boolean(args[0].is_atom()))
}

fn deref_atom(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Atom(ref val) = *args[0].mal_type {
        Ok(val.borrow().clone())
    } else {
        Err(MalError::RustFunction(
            "Invalid argument. Expected atom.".to_string(),
        ))
    }
}

fn reset_atom(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    if let Atom(ref val) = *args[0].mal_type {
        val.replace(args[1].clone());
        Ok(args[1].clone())
    } else {
        Err(MalError::RustFunction(
            "Invalid argument. Expected atom.".to_string(),
        ))
    }
}

fn swap_atom(args: &[MalValue], env: &mut Env) -> MalResult {
    arg_count_gte(args, 2)?;

    let atom = if let Atom(ref val) = *args[0].mal_type {
        val
    } else {
        return Err(MalError::RustFunction(
            "Invalid 1st argument. Expected atom.".to_string(),
        ));
    };

    if !args[1].is_function_or_macro() {
        return Err(MalError::RustFunction(
            "Invalid 2nd argument. Expected function.".to_string(),
        ));
    }

    let mut apply_args = Vec::with_capacity(args.len() - 1);
    apply_args.push(atom.borrow().clone());
    apply_args.extend_from_slice(&args[2..]);

    let result = core_apply(&args[1], &apply_args, env)?;

    atom.replace(result.clone());
    Ok(result)
}

fn throw(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_gte(args, 1)?;

    Err(MalError::Exception(args[0].clone()))
}

fn is_nil(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Nil = *args[0].mal_type {
        Ok(MalValue::new_boolean(true))
    } else {
        Ok(MalValue::new_boolean(false))
    }
}

fn is_true(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let True = *args[0].mal_type {
        Ok(MalValue::new_boolean(true))
    } else {
        Ok(MalValue::new_boolean(false))
    }
}

fn is_false(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let False = *args[0].mal_type {
        Ok(MalValue::new_boolean(true))
    } else {
        Ok(MalValue::new_boolean(false))
    }
}

fn is_symbol(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Symbol(_) = *args[0].mal_type {
        Ok(MalValue::new_boolean(true))
    } else {
        Ok(MalValue::new_boolean(false))
    }
}

fn apply(args: &[MalValue], env: &mut Env) -> MalResult {
    arg_count_gte(args, 2)?;

    let last_args_list = args.last().unwrap();

    if let List(MalList {
        vec: ref last_args, ..
    })
    | Vector(MalVector {
        vec: ref last_args, ..
    }) = *last_args_list.mal_type
    {
        let mut vec = Vec::with_capacity(args.len() + last_args.len() - 2);
        vec.extend_from_slice(&args[1..args.len() - 1]);
        vec.extend_from_slice(&last_args);

        core_apply(&args[0], &vec, env)
    } else {
        Err(MalError::RustFunction(
            "Invalid argument. Last argument of apply must be a list or vector.".to_string(),
        ))
    }
}

fn map(args: &[MalValue], env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    let function = &args[0];

    if let List(MalList { ref vec, .. }) | Vector(MalVector { ref vec, .. }) = *args[1].mal_type {
        let result_vec: Result<_, _> = vec
            .iter()
            .map(|elem| core_apply(function, slice::from_ref(elem), env))
            .collect();

        Ok(MalValue::new_list(result_vec?))
    } else {
        Err(MalError::RustFunction(
            "Invalid argument. Second argument of map must be a list or vector.".to_string(),
        ))
    }
}

fn symbol(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Str(ref str_val) = *args[0].mal_type {
        Ok(MalValue::new(Symbol(str_val.clone())))
    } else {
        Err(MalError::RustFunction(
            "Argument must be a string.".to_string(),
        ))
    }
}

fn is_keyword(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Keyword(_) = *args[0].mal_type {
        Ok(MalValue::new_boolean(true))
    } else {
        Ok(MalValue::new_boolean(false))
    }
}

fn keyword(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Str(ref str_val) = *args[0].mal_type {
        Ok(MalValue::new(Keyword(str_val.clone())))
    } else {
        Err(MalError::RustFunction(
            "Argument must be a string.".to_string(),
        ))
    }
}

fn vector(args: &[MalValue], _env: &mut Env) -> MalResult {
    Ok(MalValue::new_vector(Vec::from(args)))
}

fn is_vector(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Vector(_) = *args[0].mal_type {
        Ok(MalValue::new_boolean(true))
    } else {
        Ok(MalValue::new_boolean(false))
    }
}

fn is_sequential(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let List(_) | Vector(_) = *args[0].mal_type {
        Ok(MalValue::new_boolean(true))
    } else {
        Ok(MalValue::new_boolean(false))
    }
}

fn hash_map(args: &[MalValue], _env: &mut Env) -> MalResult {
    Ok(MalValue::new_map(MalMap::from_arguments(args)?))
}

fn is_map(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Map(_) = *args[0].mal_type {
        Ok(MalValue::new_boolean(true))
    } else {
        Ok(MalValue::new_boolean(false))
    }
}

fn assoc(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_gte(args, 1)?;

    if let Map(ref mal_map) = *args[0].mal_type {
        Ok(MalValue::new_map(mal_map.assoc(&args[1..])?))
    } else {
        Err(MalError::RustFunction(
            "First argument must be a hash map.".to_string(),
        ))
    }
}

fn dissoc(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_gte(args, 1)?;

    if let Map(ref mal_map) = *args[0].mal_type {
        Ok(MalValue::new_map(mal_map.dissoc(&args[1..])?))
    } else {
        Err(MalError::RustFunction(
            "First argument must be a hash map.".to_string(),
        ))
    }
}

fn get(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    match *args[0].mal_type {
        Map(ref mal_map) => Ok(mal_map.get(&args[1])),
        Nil => Ok(MalValue::nil()),
        _ => Err(MalError::RustFunction(
            "First argument must be a hash map.".to_string(),
        )),
    }
}

fn contains(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    match *args[0].mal_type {
        Map(ref mal_map) => Ok(MalValue::new_boolean(mal_map.contains(&args[1]))),
        Nil => Ok(MalValue::new_boolean(false)),
        _ => Err(MalError::RustFunction(
            "First argument must be a hash map.".to_string(),
        )),
    }
}

fn keys(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Map(ref mal_map) = *args[0].mal_type {
        let keys = mal_map.iter().map(|(key, _)| key.clone()).collect();
        Ok(MalValue::new_list(keys))
    } else {
        Err(MalError::RustFunction(
            "Argument must be a hash map.".to_string(),
        ))
    }
}

fn vals(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Map(ref mal_map) = *args[0].mal_type {
        let vals = mal_map.iter().map(|(_, val)| val.clone()).collect();
        Ok(MalValue::new_list(vals))
    } else {
        Err(MalError::RustFunction(
            "Argument must be a hash map.".to_string(),
        ))
    }
}

fn readline(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    if let Str(ref prompt) = *args[0].mal_type {
        let mut editor = Editor::<()>::new();

        let read_result = editor.readline(prompt);
        match read_result {
            Ok(line) => Ok(MalValue::new(Str(line.trim_end_matches('\n').to_string()))),
            Err(ReadlineError::Eof) => Ok(MalValue::nil()),
            Err(_err) => Err(MalError::RustFunction("Error reading line.".to_string())),
        }
    } else {
        Err(MalError::RustFunction(
            "Argument must be a string.".to_string(),
        ))
    }
}

fn meta(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    args[0].get_meta()
}

fn with_meta(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 2)?;

    args[0].clone_with_meta(args[1].clone())
}

fn is_string(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    Ok(MalValue::new_boolean(args[0].is_string()))
}

fn is_number(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    Ok(MalValue::new_boolean(args[0].is_number()))
}

fn is_fn(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    Ok(MalValue::new_boolean(args[0].is_function()))
}

fn is_macro(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    Ok(MalValue::new_boolean(args[0].is_macro()))
}

fn time_ms(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 0)?;

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| MalError::RustFunction("Could not calculate the current time.".to_string()))?
        .as_millis();

    Ok(MalValue::new(Number(millis as f64)))
}

fn seq(args: &[MalValue], _env: &mut Env) -> MalResult {
    arg_count_eq(args, 1)?;

    match *args[0].mal_type {
        List(MalList { ref vec, .. }) | Vector(MalVector { ref vec, .. }) if vec.is_empty() => {
            Ok(MalValue::nil())
        }
        List(_) => Ok(args[0].clone()),
        Vector(ref mal_vec) => Ok(MalValue::new_list(mal_vec.vec.clone())),
        Str(ref str_val) if str_val.is_empty() => Ok(MalValue::nil()),
        Str(ref str_val) => {
            let chars = str_val
                .chars()
                .map(|c| MalValue::new(Str(c.to_string())))
                .collect();
            Ok(MalValue::new_list(chars))
        }
        Nil => Ok(MalValue::nil()),
        _ => Err(MalError::RustFunction("Invalid argument".to_string())),
    }
}
