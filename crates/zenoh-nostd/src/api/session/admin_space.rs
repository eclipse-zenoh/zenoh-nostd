use core::fmt::Write;

use zenoh_proto::{CollectionError, SessionError, keyexpr};

use crate::{api::session::Session, config::ZSessionConfig};

impl<Config> Session<'static, Config>
where
    Config: ZSessionConfig,
{
    pub async fn start_admin_space(&'static self) -> core::result::Result<(), SessionError> {
        {
            let state = self.state().await;
            if state.admin_space_id().is_some() {
                return Err(CollectionError::KeyAlreadyExists.into());
            }
        }

        let ke = keyexpr::from_str_unchecked("@/**");

        let queryable = self
            .declare_queryable(ke)
            .callback(async move |query| {
                let mut buf = heapless::String::<128>::new();
                let _ = write!(
                    buf,
                    r#"{{"zid":"{:?}","version":"{}"}}"#,
                    query.zid(),
                    zenoh_proto::VERSION,
                );

                let _ = query.reply(query.keyexpr(), buf.as_bytes()).await;
            })
            .finish()
            .await?;

        let mut state = self.state().await;
        state.set_admin_space_id(Some(queryable.id()));

        Ok(())
    }

    pub async fn stop_admin_space(&'static self) -> core::result::Result<(), SessionError> {
        let mut state = self.state().await;
        let id = state
            .admin_space_id()
            .ok_or(CollectionError::KeyNotFound)?;
        state.set_admin_space_id(None);
        drop(state);

        self._undeclare_queryable(id).await
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Write;
    use zenoh_proto::fields::ZenohIdProto;

    #[test]
    fn admin_space_response_format() {
        let zid = ZenohIdProto::default();
        let mut buf = heapless::String::<128>::new();
        let _ = write!(buf, r#"{{"zid":"{:?}","version":"{}"}}"#, zid, zenoh_proto::VERSION);

        let s = buf.as_str();
        assert!(s.starts_with(r#"{"zid":""#));
        assert!(s.contains(r#""version":"#));
    }
}
