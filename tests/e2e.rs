use qubit::ts::router::Router;

#[qubit::handler(query)]
fn cool_handler() -> u32 {
    123
}

#[qubit::handler(query)]
fn even_cool_handler() -> u32 {
    456
}

#[test]
fn main() {
    let router = Router::new()
        .handler(cool_handler)
        .handler(even_cool_handler);

    println!("{}", router.generate_type_to_string());
}
