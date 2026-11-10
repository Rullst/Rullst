//! What `make:pwa` prints: what it wrote, what it counted, and each step it
//! left to the developer.
use super::Plan;

/// The line that mounts the PWA routes.
pub(super) const MOUNT: &str = ".merge_axum(crate::pwa::router().into_axum())";

impl Plan {
    /// The steps this run could not do; printed on every run until done.
    fn manual_steps(&self) -> Vec<String> {
        let mut steps = Vec::new();
        if self.layouts + self.present == 0 {
            steps.push("No page layout was found in src/ (an html! document with a <head>, a <head> written in a string, or render_page_with_lang), so no page registers the worker yet: put `{crate::pwa::head()}` inside the <head> of your page layout by hand.".to_owned());
        }
        if let Some(reason) = self.unmounted {
            steps.push(format!(
                "The PWA routes (/sw.js, /offline) are not mounted: {reason}. Add `{MOUNT}` to your router by hand, next to `.merge_axum(rullst::health::health_router())` and outside the CSRF layer."
            ));
        }
        steps
    }

    pub(super) fn report(&self) -> String {
        let steps = self.manual_steps();
        if self.edits.is_empty() && steps.is_empty() {
            return "The PWA files and wiring are already in place; nothing changed.\n".to_owned();
        }
        let mut report = String::from(if self.edits.is_empty() {
            "The PWA files are in place; nothing changed.\n"
        } else {
            "Progressive Web App added.\n"
        });
        for path in &self.created {
            report.push_str(&format!("  created  {path}\n"));
        }
        for path in &self.changed {
            report.push_str(&format!("  updated  {path}\n"));
        }
        for path in &self.skipped {
            report.push_str(&format!(
                "  skipped  {path}: an e-mail message, not a page layout\n"
            ));
        }
        if self.layouts > 0 {
            report.push_str(&format!(
                "{} page layout(s) now register /sw.js.\n",
                self.layouts
            ));
        }
        if self.own_manifest > 0 {
            report.push_str(&format!(
                "{} of them keep their own manifest link and received only the registration script; static/manifest.webmanifest is not linked there, so merge it into yours (PWA.md).\n",
                self.own_manifest
            ));
        }
        if self.present > 0 {
            report.push_str(&format!(
                "{} page layout(s) already register /sw.js.\n",
                self.present
            ));
        }
        for step in steps {
            report.push_str(&format!("To do: {step}\n"));
        }
        if !self.edits.is_empty() {
            if let Some(lang) = &self.untranslated {
                report.push_str(&format!(
                    "The offline page and the update notice are in English (make:pwa writes en, pt and es; --lang is {lang}): translate them in src/pwa.rs and static/pwa.js.\n"
                ));
            }
            report.push_str(
                "The icons in static/icons/ are placeholders (the Rullst logo): replace them, and read PWA.md before release.\n",
            );
        }
        report
    }
}
