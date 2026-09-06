use crate::{CredentialError, Stored, store};
use serde::{Deserialize, Serialize};

const ACCOUNT: &str = "notion-calendar";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionCalendar {
    pub view_url: String,
}

impl NotionCalendar {
    pub fn view_id(&self) -> Result<String, CredentialError> {
        let url = url::Url::parse(self.view_url.trim()).map_err(|_| {
            CredentialError::InvalidValue(
                "Notion calendar URL",
                "expected a Notion calendar view URL",
            )
        })?;
        let host = url.host_str().unwrap_or_default();
        if url.scheme() != "https"
            || !(matches!(host, "notion.so" | "notion.com" | "notion.site")
                || host.ends_with(".notion.so")
                || host.ends_with(".notion.com")
                || host.ends_with(".notion.site"))
        {
            return Err(CredentialError::InvalidValue(
                "Notion calendar URL",
                "expected an HTTPS Notion URL",
            ));
        }
        let id = url
            .query_pairs()
            .find(|(key, _)| key == "v")
            .ok_or(CredentialError::InvalidValue(
                "Notion calendar URL",
                "copy the view link including its v parameter",
            ))?
            .1
            .into_owned();
        uuid::Uuid::parse_str(&id)
            .map(|id| id.to_string())
            .map_err(|_| CredentialError::InvalidValue("Notion calendar URL", "invalid view ID"))
    }
}

pub fn notion_calendar() -> Result<Stored<NotionCalendar>, CredentialError> {
    match store::read(ACCOUNT)? {
        Stored::Missing => Ok(Stored::Missing),
        Stored::Ready(value) => {
            let configuration: NotionCalendar = serde_json::from_str(&value)?;
            configuration.view_id()?;
            Ok(Stored::Ready(configuration))
        }
    }
}

pub fn save_notion_calendar(mut configuration: NotionCalendar) -> Result<(), CredentialError> {
    configuration.view_url = configuration.view_url.trim().to_owned();
    if configuration.view_url.is_empty() {
        return store::delete(ACCOUNT);
    }
    configuration.view_id()?;
    store::save(ACCOUNT, &serde_json::to_string(&configuration)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_notion_view_links() {
        let mut configuration = NotionCalendar {
            view_url: "https://www.notion.so/calendar?v=248104cd477e80fdb757e945d38000bd".into(),
        };
        assert_eq!(
            configuration.view_id().unwrap(),
            "248104cd-477e-80fd-b757-e945d38000bd"
        );
        for url in [
            "https://notion.so/calendar",
            "https://notion.so/calendar?v=invalid",
            "https://notion.so.attacker.test/calendar?v=248104cd477e80fdb757e945d38000bd",
            "http://notion.so/calendar?v=248104cd477e80fdb757e945d38000bd",
        ] {
            configuration.view_url = url.into();
            assert!(configuration.view_id().is_err());
        }
    }
}
