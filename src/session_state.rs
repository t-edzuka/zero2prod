use actix_session::{Session, SessionExt, SessionGetError, SessionInsertError};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use std::future::{ready, Ready};
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";
    pub fn renew(&self) {
        self.0.renew()
    }

    pub fn insert_user_id(&self, user_id: Uuid) -> Result<(), SessionInsertError> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }

    pub fn get_user_id(&self) -> Result<Option<Uuid>, SessionGetError> {
        self.0.get(Self::USER_ID_KEY)
    }

    pub fn log_out(&self) {
        // Remove session for both client & server.
        self.0.purge()
    }
}

impl FromRequest for TypedSession {
    // type Error = <Session as FromRequest>::Errorは
    // TypedSessionのFromRequest実装が返すエラータイプを定義しています。
    // 具体的には、SessionのFromRequest実装が返すエラータイプと同じものを返します。
    type Error = <Session as FromRequest>::Error;
    type Future = Ready<Result<TypedSession, Self::Error>>;
    // Rustはまだトレイト内でのasync構文をサポートしていません。
    // しかし、FromRequestトレイトは非同期操作（例：HTTP呼び出し）を必要とする抽出器のために、Futureとしての戻り値を期待しています。
    // TypedSessionはI/Oを行わないため、Futureを定義、利用していません。
    // そのため、TypedSessionをReadyにラップして、
    // 最初に実行者にポーリングされたときにラップされた値に解決するFutureに変換します
    //noinspection RsBorrowChecker: 警告: use of possbly uninitilized value
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
