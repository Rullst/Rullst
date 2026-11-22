//! The English privacy notice (`/privacy.html`).

use rullst::html;
use rullst::html::Markup;

use crate::pages::{Context, Lang, blocks, made_with};

/// The English privacy notice (`/privacy.html`).
pub fn page(cx: &Context) -> Markup {
    html! {
        <!doctype html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="referrer" content="no-referrer" />
                <meta name="description" content="Rullst website privacy: responsible person, hosting, Cloudflare and email data, purposes, retention, international processing and your privacy rights." />
                <meta http-equiv="Content-Security-Policy" content="default-src 'self'; style-src 'self'; script-src 'none'; img-src 'self'; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'" />
                <title>"Website privacy notice — Rullst"</title>
                {blocks::head_links("https://rullst.win/privacy.html", "https://rullst.win/privacy.html", "https://rullst.win/pt-br/privacy.html", &cx.assets.css)}
            </head>
            <body>
                {blocks::static_nav(Lang::En, "/privacy.html", "/pt-br/privacy.html", "/#privacy")}
                <main id="main" class="shell section privacy-page">
                    <p class="eyebrow">"A small footprint. Clear information."</p>
                    <h1>"Website privacy notice."</h1>
                    <p class="privacy-updated">"Last updated: "<time datetime="2026-11-22">"November 22, 2026"</time>"."</p>
                    <p>"This notice covers the Rullst landing page and this privacy page at rullst.win and rullst.github.io, in English and Brazilian Portuguese. You can read them without creating an account. Their code uses no analytics, advertising pixels, embedded social feeds, cookies or browser storage; what Cloudflare adds to the pages is described below."</p>
                    <section id="controller" aria-labelledby="controller-heading">
                        <h2 id="controller-heading">"1. Who is responsible"</h2>
                        <p><strong>"Venelouis, based in Brazil"</strong>", is responsible for this website and acts as the controller of personal data handled for its contact and privacy requests. Contact: "<a href="mailto:officialrullst@gmail.com">"officialrullst@gmail.com"</a>". This is also the channel for privacy questions, complaints and rights requests. Public issues and social posts are not private support channels."</p>
                    </section>
                    <section id="data" aria-labelledby="data-heading">
                        <h2 id="data-heading">"2. Information involved and why"</h2>
                        <p><strong>"Website requests."</strong>" GitHub Pages hosts these pages. Serving them necessarily involves sending your IP address and technical request information, such as the requested address, time and browser information, to the host. GitHub may process this information to deliver and protect its services under the "<a href="https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement">"GitHub Privacy Statement"</a>". This website does not create visitor profiles or keep an application database of visits. We do not control GitHub's hosting logs or their retention."</p>
                        <p><strong>"Cloudflare."</strong>" Requests to rullst.win reach Cloudflare first: it sits in front of GitHub Pages as a content delivery and security service, so it receives the same IP address and request information before it passes the request to GitHub and the page back to your browser. Unless those features are turned off for the domain, Cloudflare can also change the pages it delivers: it can replace email addresses with a script that decodes them in the browser (email address obfuscation) and add a script that checks whether the browser is automated (bot detection). Its responses also ask the browser to send it reports of failed connections (Network Error Logging). These scripts and reports are Cloudflare's, not part of this website's code, and we do not control whether they set their own cookies. Cloudflare processes this information under the "<a href="https://www.cloudflare.com/privacypolicy/">"Cloudflare Privacy Policy"</a>"."</p>
                        <p><strong>"Messages you send."</strong>" If you contact us, we receive your email address, any name you provide, your message, attachments and associated email metadata through Google Gmail. We use this information to answer your inquiry, follow up on an issue or handle your privacy request. Providing it is voluntary; without enough information or a reply address, we may be unable to help. Please avoid sending passwords, identity documents or sensitive personal data unless specifically needed and requested."</p>
                        <p><strong>"Local interactions."</strong>" Navigation, animations and display preferences detected from your browser run locally. On the landing page, clicking Copy writes the displayed command to your clipboard; it does not read your clipboard or send its contents to us."</p>
                    </section>
                    <section id="purposes" aria-labelledby="purposes-heading">
                        <h2 id="purposes-heading">"3. Purposes and legal bases"</h2>
                        <p>"For ordinary correspondence we rely, where permitted, on our legitimate interest in responding to inquiries you initiate and maintaining the project, taking your rights and reasonable expectations into account. When LGPD or GDPR applies, these are the legitimate-interest grounds in LGPD Article 7(IX) and GDPR Article 6(1)(f). Handling a statutory privacy request or a binding legal obligation relies on LGPD Article 7(II) or GDPR Article 6(1)(c), as applicable."</p>
                        <p>"Where applicable law requires consent for a particular activity, it must be obtained before that activity. Merely visiting this website does not constitute consent. We do not use contact messages to enroll you in marketing. GitHub, Cloudflare and Google explain the legal bases for their own processing in their respective privacy notices."</p>
                    </section>
                    <section id="providers" aria-labelledby="providers-heading">
                        <h2 id="providers-heading">"4. Providers and international processing"</h2>
                        <p>"GitHub receives website requests as the hosting provider, and Cloudflare as the service in front of it; Google processes messages sent to the Gmail contact address. Our policy is to share information with people assisting with your inquiry only as needed to resolve it. Information may also need to be disclosed when required by law or to establish, exercise or defend legal rights."</p>
                        <p>"The responsible person is in Brazil. Provider processing may take place in the United States and other countries where GitHub, Cloudflare or Google operates, which can have different privacy laws. Their "<a href="https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement#international-data-transfers">"GitHub international transfer information"</a>" and "<a href="https://policies.google.com/privacy">"Google Privacy Policy"</a>" describe their locations and safeguards. Those provider statements do not certify Rullst's compliance. Contact us for information about the recipients and transfer arrangements relevant to your data; applicable transfer requirements must be met for processing under our responsibility."</p>
                    </section>
                    <section id="retention" aria-labelledby="retention-heading">
                        <h2 id="retention-heading">"5. Retention and security"</h2>
                        <p>"Our retention policy is to keep correspondence only for the time needed to resolve the inquiry and related follow-up. Records needed to meet a legal obligation or address a dispute may be retained for that requirement's duration. Once those purposes end, our policy requires deletion or anonymization. These criteria apply to records under our control; GitHub, Cloudflare and Google set their own retention and backup schedules for provider-managed records."</p>
                        <p>"The pages load their resources from the same website and restrict third-party content and background connections. These measures reduce unnecessary disclosure; they do not eliminate every security risk. Please use the "<a href="https://github.com/Rullst/Rullst/security/policy">"security reporting channel"</a>" for vulnerabilities rather than a public issue containing personal data."</p>
                    </section>
                    <section id="choices" aria-labelledby="choices-heading">
                        <h2 id="choices-heading">"6. Cookies, advertising and linked services"</h2>
                        <p>"The code on these pages does not set or read cookies or similar storage and does not implement fingerprinting or targeted advertising; the scripts Cloudflare may add are described in section 2. This website does not sell or share personal information for cross-context behavioral advertising. There is no optional tracking to accept or reject. This behavior is the same whether or not your browser sends a Global Privacy Control (GPC) or Do Not Track (DNT) signal."</p>
                        <p>"Social links are ordinary links: the social service is contacted when you follow one, rather than through an embedded feed on these pages. Rullst Academy, the capoeira and vene.win applications, linked documentation, benchmarks, repositories and the Showcase, LMS, Portfolio and SaaS demos are outside this notice's scope. They may use accounts, cookies, local preferences or external resources and require their own information about those activities. The documentation can store display preferences locally and include external resources; the microbenchmark pages under /Rullst/benches/ currently load Chart.js from jsDelivr. Applications built with the Rullst framework also need privacy practices appropriate to their own data flows."</p>
                    </section>
                    <section id="rights" aria-labelledby="rights-heading">
                        <h2 id="rights-heading">"7. Your rights and how to exercise them"</h2>
                        <p>"Depending on applicable law, you may request confirmation of processing, access or a copy, correction, deletion, anonymization, blocking or restriction, portability, and information about recipients. You may also have rights to object, withdraw consent where processing relies on it, or learn the consequences of refusing consent. Withdrawal does not affect earlier lawful processing. These pages do not make automated decisions with legal or similarly significant effects on you."</p>
                        <p>"Email "<a href="mailto:officialrullst@gmail.com?subject=Privacy%20request">"officialrullst@gmail.com"</a>" with your request and enough context to locate the relevant correspondence. You do not need an account. We may ask for proportionate information to verify your identity or an authorized agent's authority; please do not send identity documents unsolicited."</p>
                        <p>"Requests will be handled within the deadlines and conditions of the law that applies, normally without charge and without discrimination for exercising your rights. If a request is refused or an extension is permitted and needed, we will explain why and any available review or appeal route. You can reply to ask for review and may complain to the competent authority. For provider-controlled records, we can help identify the appropriate provider contact; we cannot directly delete its independent logs."</p>
                    </section>
                    <section id="regional-rights" aria-labelledby="regional-rights-heading">
                        <h2 id="regional-rights-heading">"8. Regional privacy information"</h2>
                        <p>"Laws have different territorial, activity and business thresholds. The following references help you find relevant rights and authorities; availability of the website worldwide does not mean every law automatically applies."</p>
                        <ul>
                            <li><strong>"Brazil — LGPD."</strong>" Rights include those in Article 18 of the "<a href="https://www.planalto.gov.br/ccivil_03/_ato2015-2018/2018/lei/l13709compilado.htm">"LGPD"</a>", subject to its conditions. You may petition the "<a href="https://www.gov.br/anpd/pt-br">"ANPD"</a>"."</li>
                            <li><strong>"European Economic Area — GDPR."</strong>" Where applicable, you have the rights described in the "<a href="https://www.edpb.europa.eu/sme/be-compliant/respect-individuals-rights_en">"EDPB rights guide"</a>" and may complain to a "<a href="https://www.edpb.europa.eu/about-edpb/our-members_en">"supervisory authority"</a>", including where you live or work."</li>
                            <li><strong>"United Kingdom — UK GDPR and Data Protection Act."</strong>" You may exercise applicable data protection rights and raise concerns with the "<a href="https://ico.org.uk/make-a-complaint/">"Information Commissioner's Office"</a>"."</li>
                            <li><strong>"California and other US states."</strong>" Where the "<a href="https://oag.ca.gov/privacy/ccpa">"CCPA, as amended by CPRA"</a>", or another state privacy law applies, rights may include access, correction, deletion, portability, and opting out of sale, sharing, targeted advertising or certain profiling. This website does not carry out those advertising, sale or profiling activities. Applicable appeal and authorized-agent rights remain available."</li>
                            <li><strong>"Canada."</strong>" Where PIPEDA or provincial privacy law applies, you may seek access, correction and review of a privacy concern. See the "<a href="https://www.priv.gc.ca/en/privacy-topics/privacy-laws-in-canada/the-personal-information-protection-and-electronic-documents-act-pipeda/p_principle/">"Office of the Privacy Commissioner of Canada"</a>" for the federal principles and complaint information."</li>
                            <li><strong>"Switzerland."</strong>" Where the Federal Act on Data Protection applies, you may exercise its information and other applicable rights. See the "<a href="https://www.edoeb.admin.ch/en/duty-to-provide-information">"Federal Data Protection and Information Commissioner"</a>"."</li>
                            <li><strong>"Australia."</strong>" Where the Privacy Act and Australian Privacy Principles apply, you may request access or correction and make a privacy complaint. See the "<a href="https://www.oaic.gov.au/privacy/australian-privacy-principles">"Office of the Australian Information Commissioner"</a>"."</li>
                            <li><strong>"Singapore."</strong>" Where the PDPA applies, relevant rights include access, correction and withdrawal of consent, subject to legal exceptions. See the "<a href="https://www.pdpc.gov.sg/data-protection-obligations">"Personal Data Protection Commission"</a>"."</li>
                        </ul>
                        <p>"If you are elsewhere, contact us to exercise the rights available under your local law. This notice does not limit mandatory legal rights."</p>
                    </section>
                    <section id="children-changes" aria-labelledby="children-changes-heading">
                        <h2 id="children-changes-heading">"9. Children and changes to this notice"</h2>
                        <p>"These pages provide general information about a software framework and do not offer child accounts or request children's personal information. If a child has sent personal information that needs attention, a parent or guardian can contact us to request review and, where appropriate, deletion."</p>
                        <p>"The date above identifies the latest revision. We will update this notice when the website's data practices change, provide additional notice where required, and obtain consent before introducing processing that requires it. This notice describes these pages and their contact channel; it is not a certification of worldwide compliance or a privacy policy for every Rullst application."</p>
                    </section>
                </main>
                <footer class="shell footer-bottom"><a href="/">"Rullst home"</a>{made_with::short(Lang::En)}<a href="mailto:officialrullst@gmail.com">"Contact the team"</a></footer>
            </body>
        </html>
    }
}
