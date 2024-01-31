use rs_ts_api::handler;

use crate::{Metadata, User};

#[handler]
fn get_user(_id: String) -> User {
    println!("get user");

    User {
        name: "some user".to_string(),
        email: "email@example.com".to_string(),
        age: 100,
        metadata: Metadata {
            param_a: String::new(),
            param_b: 123,
            param_c: true,
        },
    }
}

// #[handler(query)]
pub fn list_users() -> Vec<User> {
    todo!()
}

fn make_router() {
    // let handler = Handler::<(String,), User>::call(&get_user, serde_json::Value::Null);

    let user_router = rs_ts_api::Router::new().handler(get_user);
    //     .handler(list_users);
}
