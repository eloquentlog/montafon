use rocket::http::{ContentType, Status};

use run_test;

#[test]
fn test_login() {
    run_test(|client, _| {
        let req = client
            .post("/login")
            .header(ContentType::JSON)
            .body("{\"username\": \"u$ername\", \"password\": \"pa$$w0rd\"}");
        let mut res = req.dispatch();

        assert_eq!(res.status(), Status::Ok);
        assert!(res.body_string().unwrap().contains("Success"));
    })
}