use malrs::env::Env;
use malrs::printer::pr_str;
use malrs::reader::read_str;
use malrs::readline::Readline;
use malrs::types::MalValueType::{List, Number, RustFunc, Symbol};
use malrs::types::{MalError, MalResult, MalValue, RustFunction};

fn main() {
    let mut env = create_root_env();
    let mut readline = Readline::new();

    loop {
        match readline.readline() {
            None => break,
            Some(line) => {
                if !line.is_empty() {
                    match rep(&line, &mut env) {
                        Ok(result) => println!("{}", result),
                        Err(MalError::EmptyProgram) => {}
                        Err(mal_error) => println!("Error! {}", mal_error),
                    }
                }
            }
        }
    }

    readline.save_history();
}

fn create_root_env() -> Env {
    let mut env = Env::new();

    env.set(
        "+",
        MalValue::new(RustFunc(RustFunction(|args| {
            eval_arithmetic_operation(args, |a, b| a + b)
        }))),
    );

    env.set(
        "-",
        MalValue::new(RustFunc(RustFunction(|args| {
            eval_arithmetic_operation(args, |a, b| a - b)
        }))),
    );

    env.set(
        "*",
        MalValue::new(RustFunc(RustFunction(|args| {
            eval_arithmetic_operation(args, |a, b| a * b)
        }))),
    );

    env.set(
        "/",
        MalValue::new(RustFunc(RustFunction(|args| {
            eval_arithmetic_operation(args, |a, b| a / b)
        }))),
    );

    env
}

fn eval_arithmetic_operation(args: &[MalValue], op: fn(f64, f64) -> f64) -> MalResult {
    if args.len() != 2 {
        return Err(MalError::RustFunction(format!(
            "Expected 2 arguments, got {}",
            args.len()
        )));
    }

    let arg1 = if let Number(n) = *args.get(0).unwrap().mal_type {
        Ok(n)
    } else {
        Err(MalError::RustFunction(
            "First argument must be a number".to_string(),
        ))
    }?;

    let arg2 = if let Number(n) = *args.get(1).unwrap().mal_type {
        Ok(n)
    } else {
        Err(MalError::RustFunction(
            "Second argument must be a number".to_string(),
        ))
    }?;

    Ok(MalValue::new(Number(op(arg1, arg2))))
}

fn rep(s: &str, env: &mut Env) -> Result<String, MalError> {
    let read_val = read(s)?;
    let eval_val = eval(&read_val, env)?;
    Ok(print(&eval_val))
}

fn read(s: &str) -> MalResult {
    read_str(s)
}

fn eval(ast: &MalValue, env: &mut Env) -> MalResult {
    match *ast.mal_type {
        List(ref list) if list.is_empty() => Ok(ast.clone()),
        List(ref list) => {
            let first_arg = &list[0];

            match *first_arg.mal_type {
                Symbol(ref name) if name == "def!" => apply_special_form_def(&list[1..], env),
                Symbol(ref name) if name == "let*" => apply_special_form_let(&list[1..], env),
                _ => apply_ast(ast, env),
            }
        }
        _ => eval_ast(ast, env),
    }
}

fn print(mal_val: &MalValue) -> String {
    pr_str(mal_val)
}

fn eval_ast(ast: &MalValue, env: &mut Env) -> MalResult {
    match *ast.mal_type {
        Symbol(ref s) => env.get(&s),
        List(ref list) => {
            let evaluated_list: Result<_, _> =
                list.iter().map(|mal_val| eval(mal_val, env)).collect();

            Ok(MalValue::new(List(evaluated_list?)))
        }
        _ => Ok(ast.clone()),
    }
}

fn apply_ast(ast: &MalValue, env: &mut Env) -> MalResult {
    let evaluated_list_ast = eval_ast(ast, env)?;
    match *evaluated_list_ast.mal_type {
        List(ref evaluated_list) => {
            if let RustFunc(ref rust_function) = *evaluated_list
                .get(0)
                .expect("Evaluation of non-empty list resulted in empty list.")
                .mal_type
            {
                rust_function.0(&evaluated_list[1..])
            } else {
                Err(MalError::Evaluation(
                    "First element of a list must evaluate to a function.".to_string(),
                ))
            }
        }
        _ => panic!(
            "Evaluation of list resulted in non-list: {:?}",
            evaluated_list_ast
        ),
    }
}

fn apply_special_form_def(args: &[MalValue], env: &mut Env) -> MalResult {
    if args.len() != 2 {
        return Err(MalError::SpecialForm(format!(
            "def! expected 2 arguments, got {}",
            args.len()
        )));
    }

    let arg1 = if let Symbol(ref symbol) = *args[0].mal_type {
        Ok(symbol)
    } else {
        Err(MalError::SpecialForm(
            "def! first argument must be a valid symbol name".to_string(),
        ))
    }?;

    let arg2 = eval(&args[1], env)?;

    env.set(arg1.as_str(), arg2.clone());

    Ok(arg2)
}

fn apply_special_form_let(args: &[MalValue], env: &Env) -> MalResult {
    if args.len() != 2 {
        return Err(MalError::SpecialForm(format!(
            "let* expected 2 arguments, got {}",
            args.len()
        )));
    }

    let bindings = if let List(ref bindings) = *args[0].mal_type {
        Ok(bindings.as_slice())
    } else {
        Err(MalError::SpecialForm(
            "let* first argument must be a list".to_string(),
        ))
    }?;

    if bindings.len() % 2 != 0 {
        return Err(MalError::SpecialForm(
            "let* bindings list must have an even number of elements".to_string(),
        ));
    }

    let mut inner_env = Env::with_outer_env(env);

    for i in (0..bindings.len() - 1).step_by(2) {
        let binding_name = if let Symbol(ref symbol) = *bindings[i].mal_type {
            Ok(symbol)
        } else {
            Err(MalError::SpecialForm(
                "let* odd numbered elements of binding list must be valid symbol names".to_string(),
            ))
        }?;

        let binding_expr = eval(&bindings[i + 1], &mut inner_env)?;

        inner_env.set(binding_name.as_str(), binding_expr);
    }

    let arg2 = eval(&args[1], &mut inner_env)?;

    Ok(arg2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use malrs::types::MalError::*;

    #[test]
    fn test_empty_program() {
        let mut env = create_root_env();
        assert_eq!(rep("", &mut env), Err(EmptyProgram));
    }

    #[test]
    fn test_empty_list() {
        let mut env = create_root_env();
        assert_eq!(rep("()", &mut env), Ok("()".to_string()));
    }

    #[test]
    fn test_nested_arithmetic() {
        let mut env = create_root_env();
        assert_eq!(rep("(+ 2 (* 3 4))", &mut env), Ok("14".to_string()));
    }

    #[test]
    fn test_special_form_def() {
        let mut env = create_root_env();
        assert_eq!(
            rep("(def! str1 \"abc\")", &mut env),
            Ok("\"abc\"".to_string())
        );
        assert_eq!(rep("str1", &mut env), Ok("\"abc\"".to_string()));
    }

    #[test]
    fn test_special_form_def_evaluates_2nd_par() {
        let mut env = create_root_env();
        assert_eq!(rep("(def! x (- 5 3))", &mut env), Ok("2".to_string()));
        assert_eq!(rep("x", &mut env), Ok("2".to_string()));
    }

    #[test]
    fn test_special_form_def_symbol_to_symbol() {
        let mut env = create_root_env();
        assert_eq!(rep("(def! x 1)", &mut env), Ok("1".to_string()));
        assert_eq!(rep("(def! y x)", &mut env), Ok("1".to_string()));
        assert_eq!(rep("x", &mut env), Ok("1".to_string()));
        assert_eq!(rep("y", &mut env), Ok("1".to_string()));
    }

    #[test]
    fn test_special_form_let() {
        let mut env = create_root_env();
        assert_eq!(rep("(let* (c 2) (+ 3 c))", &mut env), Ok("5".to_string()));
    }

    #[test]
    fn test_special_form_let_multiple_bindings() {
        let mut env = create_root_env();
        assert_eq!(rep("(let* (a 2 b (+ a a) c (- b a)) (+ (* a b) c))", &mut env), Ok("10".to_string()));
    }
}
