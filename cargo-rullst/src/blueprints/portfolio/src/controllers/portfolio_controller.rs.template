use rullst::server::IntoResponse;
use rullst::response::Html;
use crate::models::profile::Profile;
use crate::models::project::Project;
use crate::models::experience::Experience;
use crate::models::skill::Skill;
use crate::pages::home;

pub async fn index() -> impl IntoResponse {
    let profile = Profile::find(1).await.unwrap_or(None).unwrap_or(Profile {
        id: 1,
        name: "Vene Light".to_string(),
        title: "Senior Rust & AI Systems Engineer".to_string(),
        subtitle: "Specializing in hyper-concurrent web backends, LLM inference pipelines, and high-throughput Rust architectures.".to_string(),
        email: "rullst@veneloius.de".to_string(),
        website: "https://rullst.github.io/".to_string(),
        avatar_url: "https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png".to_string(),
        github_url: "https://github.com/Rullst".to_string(),
        linkedin_url: "https://linkedin.com".to_string(),
    });
    let projects = Project::all().await.unwrap_or_default();
    let experiences = Experience::all().await.unwrap_or_default();
    let skills = Skill::all().await.unwrap_or_default();

    Html(home::render(&profile, &projects, &experiences, &skills))
}
