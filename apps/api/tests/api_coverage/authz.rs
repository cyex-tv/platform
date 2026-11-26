//! Who may call what, for every root field of the schema. The fields and their arguments come
//! from introspection, so a new field is swept the day it lands: called signed out, every field
//! but the public reads must answer `signed_out`; called by a signed-in account without any role,
//! every staff field must answer `forbidden`. Arguments are well-typed placeholders (IDs that
//! name nothing), so nothing is changed by the sweep.

use std::collections::BTreeMap;

use cyex_tv_db::ids;
use serde_json::{Value, json};

use crate::world::{As, World, error_code};

/// Reads anyone may make signed out (they may still answer `not_found` for the placeholders).
const PUBLIC: &[&str] = &[
    "pendingDeletion",
    "channel",
    "channelByLogin",
    "set",
    "setEmotes",
    "accountSets",
    "globalSet",
    "emote",
    "searchEmotes",
    "emoteTrending",
    "searchAccounts",
    "quickSearch",
    "accountProfile",
    "accountCosmetics",
    "account",
    "accountActivity",
    "accountActivityPage",
    "emoteActivity",
    "emoteActivityPage",
    "activityTopActors",
    "emoteChannels",
    "personalSet",
    "personalSets",
    "cosmetics",
    "userCosmetics",
    "shopItems",
    "shopItem",
    "permissions",
    "activeAnnouncement",
    "accountCollection",
    "limits",
];

/// Fields only staff (a role with the right permission, or the owner) may use.
const STAFF: &[&str] = &[
    // Queries.
    "cosmeticImage",
    "cosmeticEntitlements",
    "orders",
    "codeBatches",
    "reviewQueue",
    "emoteNotes",
    "emoteHistory",
    "accountRoles",
    "accountPermissions",
    "auditLog",
    "reportQueue",
    "accountReports",
    "adminAccount",
    "accountRestrictions",
    "accountStatus",
    "staffNotes",
    "accountSessions",
    "accountSubscriptions",
    "entitlementGraph",
    "entitlementHolders",
    "roles",
    "emoteDuplicates",
    "reviewReasons",
    "accountsByLogin",
    "announcements",
    "auditTopPeople",
    "orderTopBuyers",
    "reviewTopUploaders",
    "entitlementTopHolders",
    // Mutations.
    "createBadge",
    "updateBadge",
    "deleteBadge",
    "remakeCosmeticImage",
    "createPaint",
    "updatePaint",
    "deletePaint",
    "grantCosmetic",
    "revokeCosmetic",
    "createShopItem",
    "updateShopItem",
    "deleteShopItem",
    "refundOrder",
    "createCodeBatch",
    "disableCodeBatch",
    "approveEmote",
    "addEmoteNote",
    "deleteEmoteNote",
    "rejectEmote",
    "resolveReport",
    "takedownEmote",
    "addStaffNote",
    "deleteStaffNote",
    "placeRestriction",
    "liftRestriction",
    "restrictAccount",
    "createRole",
    "updateRole",
    "deleteRole",
    "assignRole",
    "unassignRole",
    "reviewEmotes",
    "requeueEmotes",
    "setEmoteFlags",
    "dismissReports",
    "setReviewReasons",
    "createAnnouncement",
    "updateAnnouncement",
    "endAnnouncement",
];

/// Staff mutations that check their input before the caller's permission (cheap checks first):
/// placeholders that pass those checks, so the sweep reaches the permission check.
fn valid_input(field: &str) -> Option<String> {
    let id = || ids::to_text(ids::new_id());
    let paint = r##"{ name: "Sweep", kind: LINEAR_GRADIENT, stops: [{ at: 0.0, color: "#ff0000" }, { at: 1.0, color: "#0000ff" }] }"##;
    Some(match field {
        "createPaint" => format!("mutation {{ createPaint(input: {paint}) {{ id }} }}"),
        "updatePaint" => format!(
            "mutation {{ updatePaint(id: \"{}\", input: {paint}) {{ id }} }}",
            id()
        ),
        "createShopItem" => format!(
            "mutation {{ createShopItem(input: {{ name: \"Sweep\", priceCents: 100, badgeIds: [\"{}\"] }}) {{ id }} }}",
            id()
        ),
        "updateShopItem" => format!(
            "mutation {{ updateShopItem(id: \"{}\", input: {{ name: \"Sweep\", priceCents: 100, badgeIds: [\"{}\"] }}) {{ id }} }}",
            id(),
            id()
        ),
        "createCodeBatch" => format!(
            "mutation {{ createCodeBatch(input: {{ name: \"Sweep\", prefix: \"SWEEP\", count: 1, badgeIds: [\"{}\"] }}) {{ codes }} }}",
            id()
        ),
        "createRole" => {
            "mutation { createRole(input: { name: \"Sweep\", position: 1, permissions: [] }) { id } }"
                .to_owned()
        }
        _ => return None,
    })
}

const TYPE_REF: &str =
    "kind name ofType { kind name ofType { kind name ofType { kind name ofType { kind name } } } }";

/// The schema's types by name, and the root fields of Query and Mutation.
pub struct Introspected {
    types: BTreeMap<String, Value>,
    pub roots: Vec<(&'static str, Value)>,
}

pub async fn introspect(w: &World) -> Introspected {
    let query = format!(
        "{{ __schema {{
            queryType {{ name }} mutationType {{ name }}
            types {{ kind name
                fields {{ name args {{ name defaultValue type {{ {TYPE_REF} }} }} type {{ {TYPE_REF} }} }}
                inputFields {{ name defaultValue type {{ {TYPE_REF} }} }}
                enumValues {{ name }} }} }} }}"
    );
    let r = w.gql(As::Nobody, &query, json!({})).await;
    assert!(r["errors"].is_null(), "{r}");
    let schema = &r["data"]["__schema"];
    let types: BTreeMap<String, Value> = schema["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t["name"].as_str().unwrap().to_owned(), t.clone()))
        .collect();
    let mut roots = Vec::new();
    for (op, root) in [("query", "queryType"), ("mutation", "mutationType")] {
        let name = schema[root]["name"].as_str().unwrap();
        for field in types[name]["fields"].as_array().unwrap() {
            roots.push((op, field.clone()));
        }
    }
    Introspected { types, roots }
}

impl Introspected {
    /// A GraphQL literal of type `ty`: required input fields only, placeholder IDs for strings.
    fn literal(&self, ty: &Value) -> String {
        match ty["kind"].as_str().unwrap() {
            "NON_NULL" => self.literal(&ty["ofType"]),
            "LIST" => format!("[{}]", self.literal(&ty["ofType"])),
            "ENUM" => {
                let t = &self.types[ty["name"].as_str().unwrap()];
                t["enumValues"][0]["name"].as_str().unwrap().to_owned()
            }
            "INPUT_OBJECT" => {
                let t = &self.types[ty["name"].as_str().unwrap()];
                let fields: Vec<String> = t["inputFields"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|f| f["type"]["kind"] == "NON_NULL" && f["defaultValue"].is_null())
                    .map(|f| {
                        format!(
                            "{}: {}",
                            f["name"].as_str().unwrap(),
                            self.literal(&f["type"])
                        )
                    })
                    .collect();
                format!("{{ {} }}", fields.join(", "))
            }
            _ => match ty["name"].as_str().unwrap() {
                "Int" => "1".to_owned(),
                "Float" => "1.0".to_owned(),
                "Boolean" => "false".to_owned(),
                _ => format!("\"{}\"", ids::to_text(ids::new_id())),
            },
        }
    }

    fn named(ty: &Value) -> &Value {
        if ty["ofType"].is_null() {
            ty
        } else {
            Self::named(&ty["ofType"])
        }
    }

    /// The whole document calling `field` with its required arguments.
    pub fn call(&self, op: &str, field: &Value) -> String {
        let args: Vec<String> = field["args"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["type"]["kind"] == "NON_NULL" && a["defaultValue"].is_null())
            .map(|a| {
                format!(
                    "{}: {}",
                    a["name"].as_str().unwrap(),
                    self.literal(&a["type"])
                )
            })
            .collect();
        let args = if args.is_empty() {
            String::new()
        } else {
            format!("({})", args.join(", "))
        };
        let selection = match Self::named(&field["type"])["kind"].as_str().unwrap() {
            "OBJECT" | "INTERFACE" | "UNION" => " { __typename }",
            _ => "",
        };
        format!(
            "{op} {{ {}{args}{selection} }}",
            field["name"].as_str().unwrap()
        )
    }
}

#[tokio::test]
async fn every_field_refuses_the_signed_out_and_staff_fields_refuse_everyone_else() {
    let Some(w) = World::new().await else {
        return;
    };
    let plain = w.user("7201").await;
    let schema = introspect(&w).await;
    assert!(
        schema.roots.len() > 100,
        "{} root fields",
        schema.roots.len()
    );
    let names: Vec<&str> = schema
        .roots
        .iter()
        .map(|(_, f)| f["name"].as_str().unwrap())
        .collect();
    for listed in PUBLIC.iter().chain(STAFF) {
        assert!(
            names.contains(listed),
            "{listed} is listed here but isn't in the schema any more"
        );
    }

    let mut failures = Vec::new();
    for (op, field) in &schema.roots {
        let name = field["name"].as_str().unwrap();
        let doc = valid_input(name).unwrap_or_else(|| schema.call(op, field));
        let r = w.gql(As::Nobody, &doc, json!({})).await;
        if r["errors"]
            .as_array()
            .is_some_and(|e| e.iter().any(|e| e.get("path").is_none()))
        {
            failures.push(format!(
                "{name}: the sweep's document is invalid: {doc} -> {r}"
            ));
            continue;
        }
        let code = error_code(&r);
        if PUBLIC.contains(&name) {
            if matches!(code, "signed_out" | "forbidden") {
                failures.push(format!("{name}: a public read answers {code} signed out"));
            }
        } else if code != "signed_out" {
            failures.push(format!(
                "{name}: signed out, expected signed_out, got {:?}: {r}",
                code
            ));
        }
        if STAFF.contains(&name) {
            let r = w.gql(As::Site(&plain), &doc, json!({})).await;
            let code = error_code(&r);
            if code != "forbidden" {
                failures.push(format!(
                    "{name}: an account without roles expected forbidden, got {code:?}: {r}"
                ));
            }
        }
    }
    eprintln!("{} root fields swept", schema.roots.len());
    w.close().await;
    assert!(
        failures.is_empty(),
        "{} problems:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
