//! Deckwatch plugin — provisions an nginx reverse-proxy sidecar via the Infrastructure tab.
//!
//! ## WASM entry points
//!
//! - [`metadata`] — declares the "proxy" provisionable resource; called once at load time.
//! - [`provision`] — injects an nginx sidecar when an operator adds the resource on the
//!   Application Infrastructure tab.
//! - [`deprovision`] — called when the resource is removed; returns a message explaining
//!   the reconcile behaviour.
//!
//! ## Architecture
//!
//! Association is managed at the application level via the Infrastructure tab —
//! no deployment annotation required. Deckwatch calls `provision()` to persist state
//! and sidecars, then injects them into all application deployments on the next
//! reconcile cycle.

use deckwatch_plugin_sdk::{
    ConfigField, ConfigFieldType, PluginResource,
    ResourceDeprovisionRequest, ResourceDeprovisionResult,
    ResourceProvisionRequest, ResourceProvisionResult, SidecarSpec,
};
#[cfg(target_arch = "wasm32")]
use deckwatch_plugin_sdk::PluginMetadata;
#[cfg(target_arch = "wasm32")]
use extism_pdk::*;

const DEFAULT_IMAGE: &str = "nginx:1.27-alpine";
const DEFAULT_PORT_STR: &str = "80";
const DEFAULT_PORT: i32 = 80;

// ── Resource declaration ───────────────────────────────────────────────────────

/// Declares the "proxy" provisionable resource shown on the Infrastructure tab.
///
/// Not `#[cfg(target_arch = "wasm32")]`-gated so it can be called in host-target
/// unit tests.
pub fn resource_decl() -> PluginResource {
    PluginResource {
        id: "proxy".into(),
        label: "Nginx Proxy".into(),
        icon: "mdi-server-network".into(),
        description: "Provision an nginx reverse-proxy sidecar for this application.".into(),
        singleton: true,
        fields: vec![
            ConfigField {
                key: "NGINX_IMAGE".into(),
                label: "nginx Image".into(),
                description: "nginx container image to inject as a sidecar.".into(),
                field_type: ConfigFieldType::String,
                default: Some(DEFAULT_IMAGE.into()),
                required: false,
                options: vec![],
                env_source: None,
            },
            ConfigField {
                key: "NGINX_PORT".into(),
                label: "Port".into(),
                description: "Port the nginx sidecar listens on.".into(),
                field_type: ConfigFieldType::Select,
                default: Some(DEFAULT_PORT_STR.into()),
                required: false,
                options: vec!["80".into(), "443".into(), "8080".into()],
                env_source: None,
            },
        ],
        output_keys: vec![],
    }
}

// ── Pure core: provision ───────────────────────────────────────────────────────

/// Provision an nginx proxy sidecar for an application.
///
/// Returns a [`ResourceProvisionResult`] with:
/// - `sidecars`: the `nginx-proxy` container on the selected port
/// - `deployment_annotations`: stamps `deckwatch.io/nginx-proxy: "provisioned"`
///
/// Testable on the native target without extism.
pub fn provision_inner(req: &ResourceProvisionRequest) -> ResourceProvisionResult {
    let mut result = ResourceProvisionResult::default();

    let image = req
        .fields
        .get("NGINX_IMAGE")
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| DEFAULT_IMAGE.to_string());

    let port = req
        .fields
        .get("NGINX_PORT")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(DEFAULT_PORT);

    result
        .deployment_annotations
        .insert("deckwatch.io/nginx-proxy".into(), "provisioned".into());

    result.sidecars.push(SidecarSpec {
        name: "nginx-proxy".into(),
        image,
        env: vec![],
        port: Some(port),
        cpu: Some("50m".into()),
        memory: Some("64Mi".into()),
    });

    result
}

// ── Pure core: deprovision ─────────────────────────────────────────────────────

/// Deprovision — nginx proxy is stateless; the deckwatch record is removed and the
/// sidecar disappears from deployments on the next reconcile via SSA.
pub fn deprovision_inner(_req: &ResourceDeprovisionRequest) -> ResourceDeprovisionResult {
    ResourceDeprovisionResult {
        errors: vec![],
        message: "nginx proxy sidecar will be removed from all application deployments on the next reconcile.".into(),
    }
}

// ── WASM entry points ──────────────────────────────────────────────────────────

/// Return static plugin metadata including the "proxy" resource declaration.
/// Called once by deckwatch at load time.
#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn metadata() -> FnResult<Json<PluginMetadata>> {
    Ok(Json(PluginMetadata {
        name: "nginx-proxy".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: "Provisions an nginx reverse-proxy sidecar via the Application Infrastructure tab".into(),
        provides: vec!["sidecar:nginx-proxy".into()],
        depends_on: vec![],
        optional_depends_on: vec![],
        config_schema: vec![ConfigField {
            key: "NGINX_IMAGE".into(),
            label: "nginx Image".into(),
            description: "Default nginx container image (can be overridden per-resource).".into(),
            field_type: ConfigFieldType::String,
            default: Some(DEFAULT_IMAGE.into()),
            required: false,
            options: vec![],
            env_source: None,
        }],
        resources: vec![resource_decl()],
    }))
}

/// Provision an nginx proxy sidecar. Called when an operator clicks "Add Nginx Proxy"
/// and submits the provisioning form on the Application Infrastructure tab.
#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn provision(
    Json(req): Json<ResourceProvisionRequest>,
) -> FnResult<Json<ResourceProvisionResult>> {
    Ok(Json(provision_inner(&req)))
}

/// Deprovision the nginx proxy resource. The sidecar is removed from all deployments
/// on the next reconcile cycle via server-side apply.
#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn deprovision(
    Json(req): Json<ResourceDeprovisionRequest>,
) -> FnResult<Json<ResourceDeprovisionResult>> {
    Ok(Json(deprovision_inner(&req)))
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn provision_req(fields: &[(&str, &str)]) -> ResourceProvisionRequest {
        ResourceProvisionRequest {
            application_name: "my-app".into(),
            namespace: "production".into(),
            resource_id: "proxy".into(),
            fields: fields.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    fn deprovision_req() -> ResourceDeprovisionRequest {
        ResourceDeprovisionRequest {
            application_name: "my-app".into(),
            namespace: "production".into(),
            resource_id: "proxy".into(),
            state: HashMap::new(),
            fields: HashMap::new(),
        }
    }

    #[test]
    fn resource_decl_has_expected_id_and_fields() {
        let r = resource_decl();
        assert_eq!(r.id, "proxy");
        assert_eq!(r.label, "Nginx Proxy");
        assert!(r.singleton, "nginx proxy must be a singleton resource");
        assert!(
            r.fields.iter().any(|f| f.key == "NGINX_IMAGE"),
            "NGINX_IMAGE field must be declared"
        );
        assert!(
            r.fields.iter().any(|f| f.key == "NGINX_PORT"),
            "NGINX_PORT field must be declared"
        );
        // Verify the port field is a Select with the expected options
        let port_field = r.fields.iter().find(|f| f.key == "NGINX_PORT").unwrap();
        assert!(port_field.options.contains(&"80".to_string()));
        assert!(port_field.options.contains(&"443".to_string()));
        assert!(port_field.options.contains(&"8080".to_string()));
    }

    #[test]
    fn provision_defaults_inject_sidecar_on_port_80() {
        let result = provision_inner(&provision_req(&[]));
        assert!(result.errors.is_empty(), "no errors expected: {:?}", result.errors);
        assert_eq!(result.sidecars.len(), 1);
        let sc = &result.sidecars[0];
        assert_eq!(sc.name, "nginx-proxy");
        assert_eq!(sc.image, DEFAULT_IMAGE);
        assert_eq!(sc.port, Some(DEFAULT_PORT));
    }

    #[test]
    fn provision_custom_port_is_used() {
        let result = provision_inner(&provision_req(&[("NGINX_PORT", "8080")]));
        assert_eq!(result.sidecars[0].port, Some(8080));
    }

    #[test]
    fn provision_custom_image_is_used() {
        let result = provision_inner(&provision_req(&[("NGINX_IMAGE", "nginx:1.28-alpine")]));
        assert_eq!(result.sidecars[0].image, "nginx:1.28-alpine");
    }

    #[test]
    fn provision_stamps_annotation() {
        let result = provision_inner(&provision_req(&[]));
        assert_eq!(
            result.deployment_annotations.get("deckwatch.io/nginx-proxy").map(String::as_str),
            Some("provisioned")
        );
    }

    #[test]
    fn provision_injects_no_env_vars_into_primary_container() {
        let result = provision_inner(&provision_req(&[]));
        assert!(result.state.is_empty(), "nginx proxy should not inject application env vars");
    }

    #[test]
    fn provision_sidecar_has_resource_requests() {
        let result = provision_inner(&provision_req(&[]));
        assert!(result.sidecars[0].cpu.is_some(), "cpu request must be set");
        assert!(result.sidecars[0].memory.is_some(), "memory request must be set");
    }

    #[test]
    fn deprovision_returns_non_empty_message() {
        let result = deprovision_inner(&deprovision_req());
        assert!(result.errors.is_empty());
        assert!(!result.message.is_empty(), "deprovision must return a user-visible message");
    }
}
