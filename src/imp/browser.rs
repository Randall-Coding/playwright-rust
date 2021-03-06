use crate::imp::{
    browser_context::BrowserContext,
    browser_type::{RecordHar, RecordVideo},
    core::*,
    page::Page,
    prelude::*,
    utils::{ColorScheme, Geolocation, HttpCredentials, ProxySettings, StorageState, Viewport}
};

#[derive(Debug)]
pub(crate) struct Browser {
    channel: ChannelOwner,
    version: String,
    contexts: Mutex<Vec<Weak<BrowserContext>>>
}

impl Browser {
    pub(crate) fn try_new(channel: ChannelOwner) -> Result<Self, Error> {
        let Initializer { version } = serde_json::from_value(channel.initializer.clone())?;
        Ok(Self {
            channel,
            version,
            contexts: Mutex::new(Vec::new())
        })
    }

    pub(crate) fn contexts(&self) -> Vec<Weak<BrowserContext>> {
        self.contexts.lock().unwrap().to_owned()
    }
    pub(crate) fn version(&self) -> &str { &self.version }

    pub(crate) async fn close(&self) -> Result<(), Arc<Error>> {
        let m: Str<Method> = "close".to_owned().try_into().unwrap();
        #[derive(Serialize)]
        struct CloseArgs {}
        let args = CloseArgs {};
        async fn catch(
            this: &Browser,
            m: Str<Method>,
            args: CloseArgs
        ) -> Result<Arc<Value>, Arc<Error>> {
            Ok(send_message!(this, m, args))
        }
        let result = catch(self, m, args).await;
        let err = match result {
            Ok(_) => return Ok(()),
            Err(e) => e
        };
        let _responded_error = match *err {
            Error::ErrorResponded(ref e) => e,
            _ => Err(err)?
        };
        // TODO: has been closed
        Ok(())
    }

    pub(crate) async fn new_context(
        &self,
        args: NewContextArgs<'_, '_, '_, '_, '_, '_, '_>
    ) -> Result<Weak<BrowserContext>, Arc<Error>> {
        let m: Str<Method> = "newContext".to_owned().try_into().unwrap();
        let res = send_message!(self, m, args);
        let NewContextResponse {
            context: OnlyGuid { guid }
        } = serde_json::from_value((*res).clone()).map_err(Error::Serde)?;
        let c = find_object!(self.context()?.lock().unwrap(), &guid, BrowserContext)?;
        self.contexts.lock().unwrap().push(c.clone());
        // TODO
        // context._browser = self
        // context._options = params
        Ok(c)
    }

    pub(crate) async fn new_page(
        &self,
        args: NewContextArgs<'_, '_, '_, '_, '_, '_, '_>
    ) -> Result<Weak<Page>, Arc<Error>> {
        let context = self.new_context(args).await?;
        unimplemented!()
    }
}

impl RemoteObject for Browser {
    fn channel(&self) -> &ChannelOwner { &self.channel }
    fn channel_mut(&mut self) -> &mut ChannelOwner { &mut self.channel }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Initializer {
    version: String
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NewContextArgs<'e, 'f, 'g, 'h, 'i, 'j, 'k> {
    sdk_language: &'static str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) proxy: Option<ProxySettings>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) viewport: Option<Viewport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) no_default_viewport: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ignoreHTTPSErrors")]
    pub(crate) ignore_http_errors: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "javaScriptEnabled")]
    pub(crate) js_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "bypassCSP")]
    pub(crate) bypass_csp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user_agent: Option<&'e str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) locale: Option<&'f str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timezone_id: Option<&'g str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) geolocation: Option<Geolocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) permissions: Option<&'h [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "extraHTTPHeaders")]
    pub(crate) extra_http_headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) offline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) http_credentials: Option<&'i HttpCredentials>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) device_scale_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_mobile: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) has_touch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) color_scheme: Option<ColorScheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) accept_downloads: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) chromium_sandbox: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) record_video: Option<RecordVideo<'j>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) record_har: Option<RecordHar<'k>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) storage_state: Option<StorageState>
}

impl Default for NewContextArgs<'_, '_, '_, '_, '_, '_, '_> {
    fn default() -> Self {
        Self {
            sdk_language: "rust",
            proxy: None,
            viewport: None,
            no_default_viewport: None,
            ignore_http_errors: None,
            js_enabled: None,
            bypass_csp: None,
            user_agent: None,
            locale: None,
            timezone_id: None,
            geolocation: None,
            permissions: None,
            extra_http_headers: None,
            offline: None,
            http_credentials: None,
            device_scale_factor: None,
            is_mobile: None,
            has_touch: None,
            color_scheme: None,
            accept_downloads: None,
            chromium_sandbox: None,
            record_video: None,
            record_har: None,
            storage_state: None
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewContextResponse {
    context: OnlyGuid
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imp::{browser_type::*, core::*, playwright::Playwright};

    crate::runtime_test!(new_context, {
        let driver = Driver::install().unwrap();
        let conn = Connection::run(&driver.executable()).unwrap();
        let p = Playwright::wait_initial_object(&conn).await.unwrap();
        let p = p.upgrade().unwrap();
        let chromium = p.chromium().upgrade().unwrap();
        let b = chromium.launch(LaunchArgs::default()).await.unwrap();
        let b = b.upgrade().unwrap();
        b.new_context(NewContextArgs::default()).await;
    });
}
