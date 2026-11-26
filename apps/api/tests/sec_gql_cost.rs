//! Query cost: the fan-out reads carry their own complexity weight and an operation selects at
//! most `MAX_ROOT_FIELDS` top-level fields, so one POST can't alias hundreds of searches. Every
//! document the website sends (the generated `apps/web/src/lib/gql/graphql.ts`, paint studio
//! included) must still pass depth, complexity and the root-field cap. Nothing is executed: a
//! test extension stops each request after validation, so no database is needed.

use std::sync::{Arc, Mutex};

use async_graphql::extensions::{
    Extension, ExtensionContext, ExtensionFactory, NextExecute, NextValidation,
};
use async_graphql::{Request, Response, ServerError, ValidationResult, Value};
use cyex_tv_api::gql::{self, MAX_ROOT_FIELDS};

/// Records the validated cost and stops before anything resolves.
#[derive(Clone, Default)]
struct StopAfterValidation(Arc<Mutex<Option<(usize, usize)>>>);

impl ExtensionFactory for StopAfterValidation {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(self.clone())
    }
}

#[async_trait::async_trait]
impl Extension for StopAfterValidation {
    async fn validation(
        &self,
        ctx: &ExtensionContext<'_>,
        next: NextValidation<'_>,
    ) -> Result<ValidationResult, Vec<ServerError>> {
        let result = next.run(ctx).await?;
        *self.0.lock().unwrap() = Some((result.complexity, result.depth));
        Ok(result)
    }

    async fn execute(
        &self,
        _ctx: &ExtensionContext<'_>,
        _operation_name: Option<&str>,
        _next: NextExecute<'_>,
    ) -> Response {
        Response::new(Value::Null)
    }
}

/// Validates `query` against the real schema and limits: the errors, and the cost when it passed.
async fn check(query: &str) -> (Vec<ServerError>, Option<(usize, usize)>) {
    let stop = StopAfterValidation::default();
    let schema = gql::builder().extension(stop.clone()).finish();
    let response = schema.execute(Request::new(query)).await;
    let cost = *stop.0.lock().unwrap();
    (response.errors, cost)
}

/// Every operation document in the generated website file: the text of each
/// `new TypedDocumentString(`…`)`, fragment-only documents left out.
fn website_documents() -> Vec<(String, String)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../web/src/lib/gql/graphql.ts");
    let source = std::fs::read_to_string(path).expect("the website's generated graphql.ts");
    let mut docs = Vec::new();
    for (i, part) in source
        .split("new TypedDocumentString(`")
        .skip(1)
        .enumerate()
    {
        let body = &part[..part.find('`').expect("a closing backtick")];
        let head = body.trim_start();
        if head.starts_with("fragment") {
            continue;
        }
        let name = head
            .split(|c: char| c == '(' || c == '{' || c.is_whitespace())
            .nth(1)
            .filter(|n| !n.is_empty())
            .map_or_else(|| format!("document {i}"), str::to_owned);
        docs.push((name, body.to_owned()));
    }
    docs
}

#[tokio::test]
async fn every_website_document_passes_the_limits() {
    let docs = website_documents();
    assert!(
        docs.len() > 100,
        "expected the website's documents, found {}",
        docs.len()
    );
    assert!(
        docs.iter().any(|(name, _)| name == "StudioPaints"),
        "the paint studio's documents are generated into the same file"
    );
    let mut failed = Vec::new();
    let mut heaviest = (0, String::new());
    for (name, doc) in &docs {
        let (errors, cost) = check(doc).await;
        if !errors.is_empty() {
            failed.push(format!("{name}: {:?}", errors[0].message));
            continue;
        }
        let (complexity, _) = cost.expect("validated");
        if complexity > heaviest.0 {
            heaviest = (complexity, name.clone());
        }
    }
    assert!(
        failed.is_empty(),
        "refused website documents:\n{}",
        failed.join("\n")
    );
    println!(
        "{} website documents pass; the heaviest is {} at {}",
        docs.len(),
        heaviest.1,
        heaviest.0
    );
}

#[tokio::test]
async fn aliased_searches_are_refused() {
    let one = "s0: searchEmotes(query: \"a\") { total items { id name } }";
    let (errors, _) = check(&format!("{{ {one} }}")).await;
    assert!(errors.is_empty(), "one search passes: {errors:?}");

    let many: String = (0..30)
        .map(|i| format!("s{i}: searchEmotes(query: \"a\") {{ total items {{ id name }} }} "))
        .collect();
    let (errors, _) = check(&format!("{{ {many} }}")).await;
    assert!(
        errors
            .iter()
            .any(|e| e.message.to_lowercase().contains("complex")),
        "30 aliased searches are refused for complexity: {errors:?}"
    );
}

#[tokio::test]
async fn root_fields_are_capped() {
    let fields = |n: usize| -> String { (0..n).map(|i| format!("t{i}: __typename ")).collect() };
    let (errors, _) = check(&format!("{{ {} }}", fields(MAX_ROOT_FIELDS))).await;
    assert!(
        errors.is_empty(),
        "{MAX_ROOT_FIELDS} root fields pass: {errors:?}"
    );

    let (errors, _) = check(&format!("{{ {} }}", fields(MAX_ROOT_FIELDS + 1))).await;
    let code = errors
        .first()
        .and_then(|e| e.extensions.as_ref())
        .and_then(|x| x.get("code"))
        .cloned();
    assert_eq!(code, Some(Value::from("too_many_fields")), "{errors:?}");

    // Fragments count too.
    let doc = format!(
        "query {{ ...A ...A }} fragment A on Query {{ {} }}",
        fields(MAX_ROOT_FIELDS / 2 + 1)
    );
    let (errors, _) = check(&doc).await;
    assert!(
        errors.iter().any(|e| e.message.contains("too much")),
        "fields through fragments count: {errors:?}"
    );
}
