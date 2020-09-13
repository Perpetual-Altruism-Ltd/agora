use crate::*;
use num_bigint::BigInt;

trait IntoTestResult {
    fn into(self) -> Result<BigInt, CostError>;
}

impl IntoTestResult for u64 {
    fn into(self) -> Result<BigInt, CostError> {
        Ok(BigInt::from(self))
    }
}

impl IntoTestResult for CostError {
    fn into(self) -> Result<BigInt, CostError> {
        Err(self)
    }
}

fn test(model: &str, query: &str, variables: &str, result: impl IntoTestResult) {
    let model = CostModel::compile(model).unwrap();
    let cost = model.cost(query, variables);
    assert_eq!(result.into(), cost);
}

#[test]
fn query_match() {
    let model = "
        query { a } when true => 11;
        query { b } when false => 12;
        query { b } when 1 == 1 => 2 + 2;
    ";
    test(model, "query { a }", "", 11);
    test(model, "query { b }", "", 4);
}

#[test]
fn field_args() {
    let model = "
        query { a(skip: 10) } => 15;
        query { a(skip: $skip) } when $skip > 10 => $skip * (2 + 0);
        query { a } => 55;
        query { b(skip: $skip, bob: $bob) } when $skip == $bob && true => $bob;
        query { b } => 99;
    ";
    test(model, "query { a(skip: 10) }", "", 15);
    test(model, "query { a(skip: 11) }", "", 22);
    test(model, "query { a(skip: 9) }", "", 55);
    test(model, "query { a }", "", 55);
    test(model, "query { b }", "", 99);
    test(model, "query { b(skip: 9) }", "", 99);
    test(model, "query { b(skip: 9, bob: 10) }", "", 99);
    test(model, "query { b(skip: 10, bob: 10) }", "", 10);
    test(model, "query { b(skip: 10, bob: 10), a }", "", 65);
}

#[test]
fn sums_top_levels() {
    let model = "
        query { a(skip: $skip) } => $skip;
        query { b(bob: $bob) } => 10;
        query { c } => 9;
        query { a } => 99;
        query { d } => 1;
    ";
    test(
        model,
        "query { a(skip: 10), b }",
        "",
        CostError::QueryNotCosted,
    );
    test(model, "query { a(skip: 10), b(bob: 5) }", "", 20);
    test(model, "query { a, c, d }", "", 109);
}

#[test]
fn var_substitutions() {
    let query = "query pairs($skip: Int!) { pairs(skip: $skip) { id } }";
    let variables = "{\"skip\":1}";
    let model = "query { pairs(skip: $k) } => $k;";

    let model = CostModel::compile(model).unwrap();
    let cost = model.cost(query, variables);
    assert_eq!(Ok(BigInt::from(1)), cost);
}

#[test]
fn default() {
    let query = "query { nonsense }";
    let model = "query { abc } => 2; default => 10;";
    test(model, query, "", 10);
}
