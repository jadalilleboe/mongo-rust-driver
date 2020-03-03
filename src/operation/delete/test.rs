use bson::{bson, doc};
use pretty_assertions::assert_eq;

use crate::{
    bson_util,
    cmap::{CommandResponse, StreamDescription},
    concern::{Acknowledgment, WriteConcern},
    error::{ErrorKind, WriteConcernError, WriteError, WriteFailure},
    operation::{Delete, Operation},
    options::DeleteOptions,
    Namespace,
};

#[cfg_attr(feature = "tokio-runtime", tokio::test(core_threads = 2))]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
async fn build_many() {
    let ns = Namespace {
        db: "test_db".to_string(),
        coll: "test_coll".to_string(),
    };
    let filter = doc! { "x": { "$gt": 1 } };

    let wc = WriteConcern {
        w: Some(Acknowledgment::Majority),
        ..Default::default()
    };
    let options = DeleteOptions::builder().write_concern(wc).build();

    let op = Delete::new(ns, filter.clone(), None, Some(options));

    let description = StreamDescription::new_testing();
    let mut cmd = op.build(&description).unwrap();

    assert_eq!(cmd.name.as_str(), "delete");
    assert_eq!(cmd.target_db.as_str(), "test_db");
    assert_eq!(cmd.read_pref.as_ref(), None);

    let mut expected_body = doc! {
        "delete": "test_coll",
        "deletes": [
            {
                "q": filter,
                "limit": 0,
            }
        ],
        "writeConcern": {
            "w": "majority"
        },
        "ordered": true,
    };

    bson_util::sort_document(&mut cmd.body);
    bson_util::sort_document(&mut expected_body);

    assert_eq!(cmd.body, expected_body);
}

#[cfg_attr(feature = "tokio-runtime", tokio::test(core_threads = 2))]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
async fn build_one() {
    let ns = Namespace {
        db: "test_db".to_string(),
        coll: "test_coll".to_string(),
    };
    let filter = doc! { "x": { "$gt": 1 } };

    let wc = WriteConcern {
        w: Some(Acknowledgment::Majority),
        ..Default::default()
    };
    let options = DeleteOptions::builder().write_concern(wc).build();

    let op = Delete::new(ns, filter.clone(), Some(1), Some(options));

    let description = StreamDescription::new_testing();
    let mut cmd = op.build(&description).unwrap();

    assert_eq!(cmd.name.as_str(), "delete");
    assert_eq!(cmd.target_db.as_str(), "test_db");
    assert_eq!(cmd.read_pref.as_ref(), None);

    let mut expected_body = doc! {
        "delete": "test_coll",
        "deletes": [
            {
                "q": filter,
                "limit": 1,
            }
        ],
        "writeConcern": {
            "w": "majority"
        },
        "ordered": true,
    };

    bson_util::sort_document(&mut cmd.body);
    bson_util::sort_document(&mut expected_body);

    assert_eq!(cmd.body, expected_body);
}

#[cfg_attr(feature = "tokio-runtime", tokio::test(core_threads = 2))]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
async fn handle_success() {
    let op = Delete::empty();

    let ok_response = CommandResponse::with_document(doc! {
        "ok": 1.0,
        "n": 3,
    });

    let ok_result = op.handle_response(ok_response);
    assert!(ok_result.is_ok());

    let delete_result = ok_result.unwrap();
    assert_eq!(delete_result.deleted_count, 3);
}

#[cfg_attr(feature = "tokio-runtime", tokio::test(core_threads = 2))]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
async fn handle_invalid_response() {
    let op = Delete::empty();

    let invalid_response = CommandResponse::with_document(doc! { "ok": 1.0, "asdfadsf": 123123 });
    assert!(op.handle_response(invalid_response).is_err());
}

#[cfg_attr(feature = "tokio-runtime", tokio::test(core_threads = 2))]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
async fn handle_write_failure() {
    let op = Delete::empty();

    let write_error_response = CommandResponse::with_document(doc! {
        "ok": 1.0,
        "n": 0,
        "writeErrors": [
            {
                "index": 0,
                "code": 1234,
                "errmsg": "my error string"
            }
        ]
    });
    let write_error_result = op.handle_response(write_error_response);
    assert!(write_error_result.is_err());
    match *write_error_result.unwrap_err().kind {
        ErrorKind::WriteError(WriteFailure::WriteError(ref error)) => {
            let expected_err = WriteError {
                code: 1234,
                code_name: None,
                message: "my error string".to_string(),
            };
            assert_eq!(error, &expected_err);
        }
        ref e => panic!("expected write error, got {:?}", e),
    };
}

#[cfg_attr(feature = "tokio-runtime", tokio::test(core_threads = 2))]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
async fn handle_write_concern_failure() {
    let op = Delete::empty();

    let wc_error_response = CommandResponse::with_document(doc! {
        "ok": 1.0,
        "n": 0,
        "writeConcernError": {
            "code": 456,
            "codeName": "wcError",
            "errmsg": "some message"
        }
    });

    let wc_error_result = op.handle_response(wc_error_response);
    assert!(wc_error_result.is_err());

    match *wc_error_result.unwrap_err().kind {
        ErrorKind::WriteError(WriteFailure::WriteConcernError(ref wc_error)) => {
            let expected_wc_err = WriteConcernError {
                code: 456,
                code_name: "wcError".to_string(),
                message: "some message".to_string(),
            };
            assert_eq!(wc_error, &expected_wc_err);
        }
        ref e => panic!("expected write concern error, got {:?}", e),
    }
}
