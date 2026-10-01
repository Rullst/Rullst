//! Kubernetes Manifest Blueprints for Rullst Applications

pub fn deployment_yaml(app_name: &str, port: u16) -> String {
    format!(
        r###"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {app_name}
  labels:
    app: {app_name}
spec:
  replicas: 2
  selector:
    matchLabels:
      app: {app_name}
  template:
    metadata:
      labels:
        app: {app_name}
    spec:
      containers:
        - name: {app_name}
          image: {app_name}:latest
          imagePullPolicy: IfNotPresent
          ports:
            - containerPort: {port}
          envFrom:
            - configMapRef:
                name: {app_name}-config
          readinessProbe:
            httpGet:
              path: /ready
              port: {port}
            initialDelaySeconds: 5
            periodSeconds: 10
          livenessProbe:
            httpGet:
              path: /health
              port: {port}
            initialDelaySeconds: 10
            periodSeconds: 15
          resources:
            requests:
              memory: "64Mi"
              cpu: "100m"
            limits:
              memory: "256Mi"
              cpu: "500m"
"###,
        app_name = app_name,
        port = port
    )
}

pub fn service_yaml(app_name: &str, port: u16) -> String {
    format!(
        r###"apiVersion: v1
kind: Service
metadata:
  name: {app_name}-service
  labels:
    app: {app_name}
spec:
  type: ClusterIP
  ports:
    - port: 80
      targetPort: {port}
      protocol: TCP
      name: http
  selector:
    app: {app_name}
"###,
        app_name = app_name,
        port = port
    )
}

pub fn configmap_yaml(app_name: &str, port: u16) -> String {
    format!(
        r###"apiVersion: v1
kind: ConfigMap
metadata:
  name: {app_name}-config
data:
  RULLST_ENV: "production"
  PORT: "{port}"
  RUST_LOG: "info"
"###,
        app_name = app_name,
        port = port
    )
}

pub fn hpa_yaml(app_name: &str) -> String {
    format!(
        r###"apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: {app_name}-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: {app_name}
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 50
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
"###,
        app_name = app_name
    )
}

/// cert-manager issues a certificate only for an Ingress with a `tls`
/// section; without one the production app (Secure cookies, HSTS, Nexus
/// Basic Auth over verified TLS) would be served over plain HTTP.
pub fn ingress_yaml(app_name: &str) -> String {
    format!(
        r###"apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: {app_name}-ingress
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  ingressClassName: nginx
  # Replace the placeholder host in both places with a public DNS name you
  # control; the issuer cannot certify a .local name.
  tls:
    - hosts:
        - {app_name}.local
      secretName: {app_name}-tls
  rules:
    - host: {app_name}.local
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: {app_name}-service
                port:
                  number: 80
"###,
        app_name = app_name
    )
}

pub fn all_in_one_yaml(app_name: &str, port: u16) -> String {
    format!(
        "{}\n---\n{}\n---\n{}\n---\n{}\n---\n{}",
        configmap_yaml(app_name, port),
        deployment_yaml(app_name, port),
        service_yaml(app_name, port),
        hpa_yaml(app_name),
        ingress_yaml(app_name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configmap_uses_the_canonical_environment_name() {
        let manifest = configmap_yaml("demo", 3000);
        assert!(manifest.contains("RULLST_ENV: \"production\""));
        assert!(!manifest.contains("APP_ENV:"));
    }

    #[test]
    fn ingress_requests_a_certificate_for_its_host() {
        let manifest = ingress_yaml("demo");
        // cert-manager ignores the issuer annotation without `spec.tls`.
        assert!(
            manifest.contains(
                "  tls:\n    - hosts:\n        - demo.local\n      secretName: demo-tls\n"
            )
        );
        assert!(manifest.contains("    - host: demo.local\n"));
        assert!(manifest.contains("cert-manager.io/cluster-issuer"));
        assert!(manifest.contains("  ingressClassName: nginx\n"));
        assert!(!manifest.contains("kubernetes.io/ingress.class"));
    }

    #[test]
    fn hpa_uses_the_stable_autoscaling_api() {
        let manifest = hpa_yaml("demo");
        assert!(manifest.starts_with("apiVersion: autoscaling/v2\n"));
        assert!(!manifest.contains("autoscaling/2.2"));
    }
}
