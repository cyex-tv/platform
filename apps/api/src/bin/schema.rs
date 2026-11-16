//! Prints the GraphQL SDL. CI compares it with `schema/schema.graphql`:
//! `cargo run -q -p cyex-tv-api --bin schema > schema/schema.graphql`

fn main() {
    print!("{}", cyex_tv_api::gql::sdl());
}
