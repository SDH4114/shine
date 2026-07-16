use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use shine_lang::{check_path as check_file, check_source, run_path as run_file, run_source};

#[test]
fn executes_core_language() {
    let source = r#"
fn twice(x: Int): Int { return x * 2 }
fn main() {
    values = []
    loop i in 0..6 step 2 { values.add(twice(i)) }
    assert(values == [0, 4, 8])
    assert(values[1] == 4)
    assert(values[1..] == [4, 8])
    [a, b, c] = values
    assert(a + b + c == 12)
}
"#;
    run_source(source, "core.shn").unwrap();
}

#[test]
fn supports_lists_math_and_conversions() {
    let source = r#"
fn main() {
    x: Number = 2
    x = 2.5
    values = [3, 1, 3, 2]
    assert(values.unique() == [3, 1, 2])
    values.sort()
    assert(values == [1, 2, 3, 3])
    assert(values.sum() == 9)
    assert(round(sqrt(2), 3) == 1.414)
    assert(number("42") == 42)
    assert("Amin" in ["Amin", "Murad"])
}
"#;
    run_source(source, "math.shn").unwrap();
}

#[test]
fn dynamic_variables_can_change_type() {
    let source = "value = 10\nvalue = \"text\"\nvalue = true\nassert(value)\n";
    check_source(source, "dynamic.shn").unwrap();
    run_source(source, "dynamic.shn").unwrap();
}

#[test]
fn typed_lists_enforce_mutated_elements() {
    let add_error = check_source(
        "values: List[Float] = [1.0]\nvalues.add(2)\n",
        "typed-list.shn",
    )
    .unwrap_err();
    assert_eq!(add_error.category, "Type Error");

    let index_error = run_source(
        "values: List[String] = [\"one\"]\nvalues[0] = 2\n",
        "typed-list.shn",
    )
    .unwrap_err();
    assert_eq!(index_error.category, "Type Error");
}

#[test]
fn check_does_not_execute_dynamic_functions() {
    let source = r#"
fn divide(a, b) { return a / b }
fn average(values) { return values.mean() }
fn main() { print("ready") }
"#;
    check_source(source, "dynamic-functions.shn").unwrap();
}

#[test]
fn check_reports_fixed_type_and_unknown_names() {
    let type_error = check_source("age: Int = 16\nage = \"17\"\n", "bad.shn").unwrap_err();
    assert_eq!(type_error.category, "Type Error");
    let name_error = check_source("print(missing)\n", "bad.shn").unwrap_err();
    assert_eq!(name_error.category, "Name Error");
    let interpolation_error = check_source("print(\"Value: {missing}\")\n", "bad.shn").unwrap_err();
    assert_eq!(interpolation_error.category, "Name Error");
}

#[test]
fn const_list_cannot_be_mutated() {
    let error = check_source("const values = [1]\nvalues.add(2)\n", "const.shn").unwrap_err();
    assert_eq!(error.category, "Const Error");
}

#[test]
fn all_four_loop_forms_work() {
    let source = r#"
fn main() {
    foreverCount = 0
    loop {
        foreverCount += 1
        if foreverCount == 2 { break }
    }
    whileCount = 0
    loop whileCount < 3 { whileCount += 1 }
    names = ""
    loop name in ["A", "B"] { names = names + name }
    rangeSum = 0
    loop i in 10..0 step -2 {
        if i == 6 { continue }
        rangeSum += i
    }
    assert(foreverCount == 2)
    assert(whileCount == 3)
    assert(names == "AB")
    assert(rangeSum == 24)
}
"#;
    run_source(source, "loops.shn").unwrap();
}

#[test]
fn cli_new_run_check_build_and_test() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("shine-e2e-{suffix}"));
    let shine = env!("CARGO_BIN_EXE_shine");
    let output = Command::new(shine)
        .args(["new", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let main = root.join("src/main.shn");
    let output = Command::new(shine)
        .args(["run", main.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, Shine\n");
    assert!(Command::new(shine)
        .args(["check", main.to_str().unwrap()])
        .status()
        .unwrap()
        .success());
    fs::write(
        root.join("tests/basic.shn"),
        "fn main() { assert(2 ** 3 == 8.0) }\n",
    )
    .unwrap();
    assert!(Command::new(shine)
        .args(["test", root.to_str().unwrap()])
        .status()
        .unwrap()
        .success());
    let build_dir = root.join("build-work");
    fs::create_dir_all(&build_dir).unwrap();
    assert!(Command::new(shine)
        .current_dir(&build_dir)
        .args(["build", main.to_str().unwrap()])
        .status()
        .unwrap()
        .success());
    let artifact = build_dir.join("target/shine/main");
    let output = Command::new(&artifact).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, Shine\n");
    fs::remove_dir_all(root).ok();
}

#[test]
fn cli_creates_relative_project_from_any_working_directory() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let workspace = std::env::temp_dir().join(format!("shine-anywhere-{suffix}"));
    fs::create_dir_all(&workspace).unwrap();
    let shine = env!("CARGO_BIN_EXE_shine");

    let output = Command::new(shine)
        .current_dir(&workspace)
        .args(["new", "demo"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let project = workspace.join("demo");
    assert!(project.join("shine.toml").is_file());
    assert!(project.join("src/main.shn").is_file());
    let output = Command::new(shine)
        .current_dir(&project)
        .args(["run", "src/main.shn"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, Shine\n");
    fs::remove_dir_all(workspace).ok();
}

#[test]
fn documentation_examples_check_and_run() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/examples");
    let mut examples: Vec<_> = fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "shn"))
        .collect();
    examples.sort();

    for example in examples {
        let source = fs::read_to_string(&example).unwrap();
        let file = example.display().to_string();
        check_source(&source, &file).unwrap_or_else(|error| panic!("{file}:\n{error}"));
        run_source(&source, &file).unwrap_or_else(|error| panic!("{file}:\n{error}"));
    }
}

#[test]
fn formatter_preserves_multiline_string_contents() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("shine-format-{suffix}"));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("multiline.shn");
    fs::write(
        &source,
        "fn main() {\ntext = \"\"\"\n  this is string content\n\"\"\"\nprint(text)\n}\n",
    )
    .unwrap();

    let shine = env!("CARGO_BIN_EXE_shine");
    let output = Command::new(shine)
        .args(["fmt", source.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let formatted = fs::read_to_string(&source).unwrap();
    assert!(formatted.contains("\n  this is string content\n"));
    assert!(Command::new(shine)
        .args(["run", source.to_str().unwrap()])
        .status()
        .unwrap()
        .success());
    fs::remove_dir_all(root).ok();
}

#[test]
fn multi_module_imports_check_run_and_build() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("shine-modules-{suffix}"));
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("main.shn"),
        r#"import math as numbers
from labels import label

fn main() {
    assert(numbers.double(4) == 8)
    assert(numbers.summary(4) == "result=8")
    scale = numbers.Scale(3)
    assert(scale.apply(4) == 12)
    print(label())
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("math.shn"),
        "const factor = 2\nexport fn double(value: Int): Int { return value * factor }\nexport fn summary(value: Int): String { return \"result={double(value)}\" }\nexport class Scale { factor = 1\nfn init(factor) { self.factor = factor }\nfn apply(value) { return value * self.factor }\n}\nfn hidden() { return 0 }\n",
    )
    .unwrap();
    fs::write(
        src.join("labels.shn"),
        "export fn label(): String { return \"modules work\" }\n",
    )
    .unwrap();

    let entry = src.join("main.shn");
    let hir = check_file(&entry).unwrap();
    assert_eq!(hir.module_count, 3);
    run_file(&entry).unwrap();

    let build_dir = root.join("build");
    fs::create_dir_all(&build_dir).unwrap();
    let shine = env!("CARGO_BIN_EXE_shine");
    let output = Command::new(shine)
        .current_dir(&build_dir)
        .args(["build", entry.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact = build_dir.join("target/shine/main");
    let output = Command::new(artifact).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "modules work\n");
    fs::remove_dir_all(root).ok();
}

#[test]
fn modules_enforce_exports_and_reject_cycles() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("shine-module-errors-{suffix}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.shn"),
        "import helpers\nfn main() { helpers.secret() }\n",
    )
    .unwrap();
    fs::write(root.join("helpers.shn"), "fn secret() { return 1 }\n").unwrap();
    let private = check_file(&root.join("main.shn")).unwrap_err();
    assert_eq!(private.category, "Module Error");
    assert!(private.message.contains("private"));

    fs::write(
        root.join("main.shn"),
        "import helpers\nfn main() { print(helpers.value()) }\n",
    )
    .unwrap();
    fs::write(
        root.join("helpers.shn"),
        "import main\nexport fn value() { return 1 }\n",
    )
    .unwrap();
    let cycle = check_file(&root.join("main.shn")).unwrap_err();
    assert_eq!(cycle.category, "Module Error");
    assert!(cycle.message.contains("cyclic import"));
    fs::remove_dir_all(root).ok();
}

#[test]
fn important_math_is_available_without_imports() {
    let source = r#"
fn main() {
    assert(TAU == 2 * PI)
    assert(round(PHI, 3) == 1.618)
    assert(round(exp(1), 10) == round(E, 10))
    assert(cbrt(27) == 3)
    assert(round(degrees(PI), 5) == 180.0)
    assert(round(radians(180), 5) == round(PI, 5))
    assert(hypot(3, 4) == 5)
    assert(sign(-8) == -1)
    assert(clamp(12, 0, 10) == 10)
    assert(gcd(54, 24) == 6)
    assert(lcm(6, 8) == 24)
    assert(factorial(5) == 120)
    values = [1, 2, 2, 3]
    assert(product(values) == 12)
    assert(mean(values) == 2)
    assert(median(values) == 2)
    assert(mode(values) == 2)
    assert(variance(values) == 0.5)
    assert(round(std(values), 5) == 0.70711)
    assert(values.median() == 2)
    assert(isFinite(PI))
    assert(isInfinite(INF))
    assert(isNan(NAN))
}
"#;
    check_source(source, "math-builtins.shn").unwrap();
    run_source(source, "math-builtins.shn").unwrap();
}

#[test]
fn constants_infer_type_while_variables_keep_both_forms() {
    let source = r#"
const GRAVITY = 9.80665
typed: Float = 9.80665
dynamic = 9.80665
typed = 10.0
dynamic = "changed"
assert(GRAVITY > 9.8)
"#;
    check_source(source, "const-simple.shn").unwrap();
    run_source(source, "const-simple.shn").unwrap();

    let error = check_source("const GRAVITY: Float = 9.80665\n", "const-typed.shn").unwrap_err();
    assert_eq!(error.category, "Syntax Error");
    assert!(error.message.contains("do not use type annotations"));
}

#[test]
fn python_like_classes_support_fields_methods_and_real_privacy() {
    let source = r#"
class Counter {
    value = 0
    private secret = 7

    fn init(start) {
        self.value = start
    }

    fn add(amount) {
        self.value += amount
        return self.value
    }

    private fn hidden() {
        return self.secret
    }

    fn reveal() {
        return self.hidden()
    }
}

fn main() {
    counter = Counter(10)
    assert(counter.value == 10)
    assert(counter.add(5) == 15)
    assert(counter.reveal() == 7)
    assert("value={counter.value}" == "value=15")
}
"#;
    check_source(source, "classes.shn").unwrap();
    run_source(source, "classes.shn").unwrap();

    let private_field = run_source(
        "class Box { private value = 1 }\nbox = Box()\nprint(box.value)\n",
        "private-field.shn",
    )
    .unwrap_err();
    assert_eq!(private_field.category, "Access Error");

    let private_method = run_source(
        "class Box { private fn value() { return 1 } }\nbox = Box()\nbox.value()\n",
        "private-method.shn",
    )
    .unwrap_err();
    assert_eq!(private_method.category, "Access Error");

    let constant_object = run_source(
        "class Box { value = 1\nfn change() { self.value = 2 }\n}\nconst box = Box()\nbox.change()\n",
        "constant-object.shn",
    )
    .unwrap_err();
    assert_eq!(constant_object.category, "Const Error");
}

#[test]
fn optimized_compound_math_preserves_overflow_diagnostics() {
    let source = "value = 9_223_372_036_854_775_807\nvalue += 1\n";
    let error = run_source(source, "compound-overflow.shn").unwrap_err();
    assert_eq!(error.category, "Value Error");
    assert!(error.message.contains("integer overflow"));
}
