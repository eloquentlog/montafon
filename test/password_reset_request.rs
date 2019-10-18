use fourche::queue::Queue;
use rocket::http::{ContentType, Status};
use redis::{Commands, RedisError};

use eloquentlog_backend_api::model;
use eloquentlog_backend_api::job;

use {run_test, load_user, USERS};

#[test]
fn test_password_reset_request_with_validation_error() {
    run_test(|client, conn, _, logger| {
        let u = USERS.get("oswald").unwrap().clone();
        let user = load_user(u, conn.db);

        let email = "invalid";
        let res = client
            .put("/_api/passordlreset")
            .header(ContentType::JSON)
            .body(format!(
                r#"{{
                  "email": "{}"
                }}"#,
                &email,
            ))
            .dispatch();

        assert_eq!(res.status(), Status::NotFound);
        let result =
            model::user::User::find_by_email(&user.email, conn.db, logger);
        assert!(result.unwrap().reset_password_token.is_none());
    });
}

#[test]
fn test_password_reset_request() {
    run_test(|client, conn, _, logger| {
        let u = USERS.get("oswald").unwrap().clone();
        let user = load_user(u, conn.db);

        let email = user.email;
        let res = client
            .put("/_api/password/reset")
            .header(ContentType::JSON)
            .body(format!(
                r#"{{
                  "email": "{}"
                }}"#,
                &email,
            ))
            .dispatch();

        assert_eq!(res.status(), Status::Ok);

        let result = model::user::User::find_by_email(&email, conn.db, logger);
        assert!(result.unwrap().reset_password_token.is_some());

        // TODO: check sent email
        let mut queue = Queue::new("default", conn.mq);
        let job = queue.dequeue::<job::Job<String>>().ok().unwrap();
        assert_eq!(job.kind, job::JobKind::SendPasswordResetEmail);
        assert!(!job.args.is_empty());

        let session_id = job.args[1].to_string();
        dbg!(&session_id);
        let value: Result<String, RedisError> = conn.ss.get(session_id);
        assert!(value.is_ok());
    });
}
