//! # Examples
//!
//! ```rust,no_run
//! use std::{thread, time};
//!
//! use ashpd::desktop::{
//!     notification::{Action, Button, Notification, NotificationProxy, Priority},
//!     Icon,
//! };
//! use zbus::zvariant::Value;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let connection = zbus::Connection::session().await?;
//!     let proxy = NotificationProxy::new(&connection).await?;
//!
//!     let notification_id = "org.gnome.design.Contrast";
//!     proxy
//!         .add_notification(
//!             notification_id,
//!             Notification::new("Contrast")
//!                 .default_action("open")
//!                 .default_action_target(Value::U32(100).into())
//!                 .body("color copied to clipboard")
//!                 .priority(Priority::High)
//!                 .icon(Icon::from_names(&["dialog-question-symbolic"]))
//!                 .button(Button::new("Copy", "copy").target(Value::U32(32).into()))
//!                 .button(Button::new("Delete", "delete").target(Value::U32(40).into())),
//!         )
//!         .await?;
//!
//!     let action = proxy.receive_action_invoked().await?;
//!     match action.name() {
//!         "copy" => (),   // Copy something to clipboard
//!         "delete" => (), // Delete the file
//!         _ => (),
//!     };
//!     println!("{:#?}", action.id());
//!     println!(
//!         "{:#?}",
//!         action.parameter().get(0).unwrap().downcast_ref::<u32>()
//!     );
//!
//!     proxy.remove_notification(notification_id).await?;
//!     Ok(())
//! }
//! ```

use std::{fmt, str::FromStr};

use serde::{self, Deserialize, Serialize};
use zbus::zvariant::{DeserializeDict, OwnedValue, SerializeDict, Type};

use super::{Icon, DESTINATION, PATH};
use crate::{
    helpers::{call_method, receive_signal},
    Error,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[zvariant(signature = "s")]
/// The notification priority
pub enum Priority {
    /// Low.
    Low,
    /// Normal.
    Normal,
    /// High.
    High,
    /// Urgent.
    Urgent,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Normal => write!(f, "Normal"),
            Self::High => write!(f, "High"),
            Self::Urgent => write!(f, "Urgent"),
        }
    }
}

impl AsRef<str> for Priority {
    fn as_ref(&self) -> &str {
        match self {
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
            Self::Urgent => "Urgent",
        }
    }
}

impl From<Priority> for &'static str {
    fn from(d: Priority) -> Self {
        match d {
            Priority::Low => "Low",
            Priority::Normal => "Normal",
            Priority::High => "High",
            Priority::Urgent => "Urgent",
        }
    }
}

impl FromStr for Priority {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Low" | "low" => Ok(Priority::Low),
            "Normal" | "normal" => Ok(Priority::Normal),
            "High" | "high" => Ok(Priority::High),
            "Urgent" | "urgent" => Ok(Priority::Urgent),
            _ => Err(Error::ParseError("Failed to parse priority, invalid value")),
        }
    }
}

#[derive(SerializeDict, Type, Debug)]
/// A notification
#[zvariant(signature = "dict")]
pub struct Notification {
    /// User-visible string to display as the title.
    title: String,
    /// User-visible string to display as the body.
    body: Option<String>,
    /// Serialized icon (e.g using gio::Icon::serialize).
    icon: Option<Icon>,
    /// The priority for the notification.
    priority: Option<Priority>,
    /// Name of an action that is exported by the application.
    /// This action will be activated when the user clicks on the notification.
    #[zvariant(rename = "default-action")]
    default_action: Option<String>,
    /// Target parameter to send along when activating the default action.
    #[zvariant(rename = "default-action-target")]
    default_action_target: Option<OwnedValue>,
    /// Array of buttons to add to the notification.
    buttons: Option<Vec<Button>>,
}

impl Notification {
    /// Create a new notification.
    ///
    /// # Arguments
    ///
    /// * `title` - the notification title.
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_owned(),
            body: None,
            priority: None,
            icon: None,
            default_action: None,
            default_action_target: None,
            buttons: None,
        }
    }

    /// Sets the notification body.
    #[must_use]
    pub fn body(mut self, body: &str) -> Self {
        self.body = Some(body.to_owned());
        self
    }

    /// Sets an icon to the notification.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets the notification priority.
    #[must_use]
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Sets the default action when the user clicks on the notification.
    #[must_use]
    pub fn default_action(mut self, default_action: &str) -> Self {
        self.default_action = Some(default_action.to_owned());
        self
    }

    /// Sets a value to be sent in the `action_invoked` signal.
    #[must_use]
    pub fn default_action_target(mut self, default_action_target: OwnedValue) -> Self {
        self.default_action_target = Some(default_action_target);
        self
    }

    /// Adds a new button to the notification.
    #[must_use]
    pub fn button(mut self, button: Button) -> Self {
        match self.buttons {
            Some(ref mut buttons) => buttons.push(button),
            None => {
                self.buttons.replace(vec![button]);
            }
        };
        self
    }
}

#[derive(SerializeDict, DeserializeDict, Type, Debug)]
/// A notification button
#[zvariant(signature = "dict")]
pub struct Button {
    /// User-visible label for the button. Mandatory.
    label: String,
    /// Name of an action that is exported by the application. The action will
    /// be activated when the user clicks on the button.
    action: String,
    /// Target parameter to send along when activating the action.
    target: Option<OwnedValue>,
}

impl Button {
    /// Create a new notification button.
    ///
    /// # Arguments
    ///
    /// * `label` - the user visible label of the button.
    /// * `action` - the action name to be invoked when the user clicks on the
    ///   button.
    pub fn new(label: &str, action: &str) -> Self {
        Self {
            label: label.to_owned(),
            action: action.to_owned(),
            target: None,
        }
    }

    /// The value to send with the action name when the button is clicked.
    #[must_use]
    pub fn target(mut self, target: OwnedValue) -> Self {
        self.target = Some(target);
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Type)]
/// An invoked action.
pub struct Action(String, String, Vec<OwnedValue>);

impl Action {
    /// Notification ID.
    pub fn id(&self) -> &str {
        &self.0
    }

    /// Action name.
    pub fn name(&self) -> &str {
        &self.1
    }

    /// The parameters passed to the action.
    pub fn parameter(&self) -> &Vec<OwnedValue> {
        &self.2
    }
}

/// The interface lets sandboxed applications send and withdraw notifications.
///
/// It is not possible for the application to learn if the notification was
/// actually presented to the user. Not a portal in the strict sense, since
/// there is no user interaction.
///
/// **Note** in contrast to most other portal requests, notifications are
/// expected to outlast the running application. If a user clicks on a
/// notification after the application has exited, it will get activated again.
///
/// Notifications can specify actions that can be activated by the user.
/// Actions whose name starts with 'app.' are assumed to be exported and will be
/// activated via the ActivateAction() method in the org.freedesktop.Application
/// interface. Other actions are activated by sending the
///  `#org.freedeskop.portal.Notification::ActionInvoked` signal to the
/// application.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.Notification`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Notification).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.Notification")]
pub struct NotificationProxy<'a>(zbus::Proxy<'a>);

impl<'a> NotificationProxy<'a> {
    /// Create a new instance of [`NotificationProxy`].
    pub async fn new(connection: &zbus::Connection) -> Result<NotificationProxy<'a>, Error> {
        let proxy = zbus::ProxyBuilder::new_bare(connection)
            .interface("org.freedesktop.portal.Notification")?
            .path(PATH)?
            .destination(DESTINATION)?
            .build()
            .await?;
        Ok(Self(proxy))
    }

    /// Get a reference to the underlying Proxy.
    pub fn inner(&self) -> &zbus::Proxy<'_> {
        &self.0
    }

    /// Signal emitted when a particular action is invoked.
    ///
    /// # Specifications
    ///
    /// See also [`ActionInvoked`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-Notification.ActionInvoked).
    #[doc(alias = "ActionInvoked")]
    #[doc(alias = "XdpPortal::notification-action-invoked")]
    pub async fn receive_action_invoked(&self) -> Result<Action, Error> {
        receive_signal(self.inner(), "ActionInvoked").await
    }

    /// Sends a notification.
    ///
    /// The ID can be used to later withdraw the notification.
    /// If the application reuses the same ID without withdrawing, the
    /// notification is replaced by the new one.
    ///
    /// # Arguments
    ///
    /// * `id` - Application-provided ID for this notification.
    /// * `notification` - The notification.
    ///
    /// # Specifications
    ///
    /// See also [`AddNotification`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Notification.AddNotification).
    #[doc(alias = "AddNotification")]
    #[doc(alias = "xdp_portal_add_notification")]
    pub async fn add_notification(
        &self,
        id: &str,
        notification: Notification,
    ) -> Result<(), Error> {
        call_method(self.inner(), "AddNotification", &(id, notification)).await
    }

    /// Withdraws a notification.
    ///
    /// # Arguments
    ///
    /// * `id` - Application-provided ID for this notification.
    ///
    /// # Specifications
    ///
    /// See also [`RemoveNotification`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Notification.RemoveNotification).
    #[doc(alias = "RemoveNotification")]
    #[doc(alias = "xdp_portal_remove_notification")]
    pub async fn remove_notification(&self, id: &str) -> Result<(), Error> {
        call_method(self.inner(), "RemoveNotification", &(id)).await
    }
}
