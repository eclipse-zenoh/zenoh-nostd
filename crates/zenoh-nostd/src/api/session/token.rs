use zenoh_proto::{exts::QoS, fields::*, msgs::*, *};

use crate::{api::session::Session, config::ZSessionConfig, io::transport::ZTransportLinkTx};

/// A liveliness token. It is visible to peers (`liveliness().get/subscribe`) until it is
/// undeclared or the session closes.
pub struct Token<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    id: u32,
    session: &'a Session<'res, Config>,
}

impl<'a, 'res, Config> Token<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub async fn undeclare(self) -> core::result::Result<(), SessionError> {
        let msg = Declare {
            body: DeclareBody::UndeclareToken(UndeclareToken {
                id: self.id,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))
            .await?;

        Ok(())
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    /// Declares a liveliness token on `ke`. The key expression is only borrowed while the
    /// declaration is sent.
    pub async fn declare_token(
        &self,
        ke: &keyexpr,
    ) -> core::result::Result<Token<'_, 'res, Config>, SessionError> {
        let id = self.state().await.next();

        let msg = Declare {
            body: DeclareBody::DeclareToken(DeclareToken {
                id,
                wire_expr: WireExpr::from(ke),
            }),
            ..Default::default()
        };

        self.driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))
            .await?;

        Ok(Token { id, session: self })
    }
}
