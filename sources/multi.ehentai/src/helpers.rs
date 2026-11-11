use crate::USER_AGENT;
use crate::settings::{
	build_cookie_header, get_base_url, get_domain, get_ipb_member_id, get_ipb_pass_hash,
	refresh_igneous_from_set_cookie,
};
use aidoku::{
	Result,
	alloc::{Vec, string::String, string::ToString},
	imports::{
		html::Document,
		net::{Request, Response},
	},
	prelude::*,
};

/// Lowercased host of an absolute URL (no userinfo, port, path or query).
fn url_host(url: &str) -> Option<String> {
	let authority = url.split_once("://")?.1.split(['/', '?', '#']).next()?;
	let host = authority.rsplit('@').next()?.split(':').next()?;
	Some(host.to_ascii_lowercase())
}

fn is_host(url: &str, domain: &str) -> bool {
	url_host(url).is_some_and(|h| h == domain || h.ends_with(&format!(".{domain}")))
}

pub fn is_exhentai(url: &str) -> bool {
	is_host(url, "exhentai.org")
}

/// Session cookies (ipb_member_id / ipb_pass_hash) may only go to E-Hentai itself:
/// gallery images are served by third-party Hentai@Home nodes (*.hath.network).
pub fn is_eh_host(url: &str) -> bool {
	is_host(url, "e-hentai.org") || is_exhentai(url)
}

/// GET with E-Hentai headers; cookies are attached only for E-Hentai hosts.
pub fn eh_request(url: &str, cookies: &str, referer: Option<&str>) -> Result<Request> {
	let mut req = Request::get(url)?.header("User-Agent", USER_AGENT);
	if let Some(referer) = referer {
		req = req.header("Referer", referer);
	}
	if is_eh_host(url) {
		req = req.header("Cookie", cookies);
	}
	Ok(req)
}

pub fn eh_get_html(url: &str, cookies: &str, user_agent: &str) -> Result<Document> {
	let do_request = |cookie_header: &str| -> Result<Response> {
		let mut req = Request::get(url)?.header("User-Agent", user_agent);
		if is_eh_host(url) {
			req = req.header("Cookie", cookie_header);
		}
		Ok(req.send()?)
	};

	let mut resp = do_request(cookies)?;

	if is_exhentai(url)
		&& let Some(set_cookie) = resp.get_header("Set-Cookie")
	{
		refresh_igneous_from_set_cookie(&set_cookie);
	}

	let doc = resp.get_html()?;

	if is_exhentai(url) {
		let is_rejected = doc
			.select_first("body")
			.and_then(|b| b.select_first("div"))
			.is_none();

		if is_rejected {
			let base_cookie = {
				let member_id = get_ipb_member_id();
				let pass_hash = get_ipb_pass_hash();
				let mut parts: Vec<String> = Vec::new();
				parts.push("nw=1".into());
				if !member_id.is_empty() {
					parts.push(format!("ipb_member_id={}", member_id));
				}
				if !pass_hash.is_empty() {
					parts.push(format!("ipb_pass_hash={}", pass_hash));
				}
				parts.join("; ")
			};

			if let Ok(probe_req) = Request::get("https://exhentai.org")
				&& let Ok(probe) = probe_req
					.header("Cookie", &base_cookie)
					.header("User-Agent", user_agent)
					.send() && let Some(set_cookie) = probe.get_header("Set-Cookie")
			{
				refresh_igneous_from_set_cookie(&set_cookie);
			}

			let refreshed_cookies = build_cookie_header();
			resp = do_request(&refreshed_cookies)?;

			if let Some(set_cookie) = resp.get_header("Set-Cookie") {
				refresh_igneous_from_set_cookie(&set_cookie);
			}

			let retry_doc = resp.get_html()?;
			let still_rejected = retry_doc
				.select_first("body")
				.and_then(|b| b.select_first("div"))
				.is_none();
			if still_rejected {
				bail!(
					"Access denied by ExHentai. Please check your account permissions or re-login."
				);
			}
			return Ok(retry_doc);
		}
	}

	Ok(doc)
}

pub fn rewrite_domain(url: &str) -> String {
	let domain = get_domain();
	if !is_eh_host(url) {
		url.to_string()
	} else if url.contains("exhentai.org") {
		url.replacen("exhentai.org", &domain, 1)
	} else if url.contains("e-hentai.org") {
		url.replacen("e-hentai.org", &domain, 1)
	} else {
		url.to_string()
	}
}

pub fn get_api_url() -> &'static str {
	"https://api.e-hentai.org/api.php"
}

pub fn api_showpage(
	gid: &str,
	imgkey: &str,
	page: u32,
	showkey: &str,
	nl: Option<&str>,
	cookies: &str,
) -> Option<(String, Option<String>)> {
	let nl_val = nl.unwrap_or("");
	let body = format!(
		r#"{{"method":"showpage","gid":{gid},"imgkey":"{imgkey}","page":{page},"showkey":"{showkey}","nl":"{nl_val}"}}"#
	);
	let mut resp = Request::post(get_api_url())
		.ok()?
		.header("Content-Type", "application/json")
		.header("Cookie", cookies)
		.header("User-Agent", USER_AGENT)
		.header("Referer", &get_base_url())
		.body(body.as_bytes())
		.send()
		.ok()?;

	let json: serde_json::Value = resp.get_json().ok()?;
	let i3 = json.get("i3").and_then(|v| v.as_str())?;
	let img_url = extract_src_from_img_html(i3)?;
	let nl_out = json
		.get("i6")
		.and_then(|v| v.as_str())
		.and_then(extract_nl_from_i6);

	Some((img_url.to_string(), nl_out))
}

pub fn extract_src_from_img_html(html: &str) -> Option<&str> {
	let start = html
		.find("src=\"")
		.map(|i| (i + 5, '"'))
		.or_else(|| html.find("src='").map(|i| (i + 5, '\'')))?;
	let (idx, quote) = start;
	let end = html[idx..].find(quote)?;
	Some(&html[idx..idx + end])
}

pub fn extract_nl_from_i6(i6: &str) -> Option<String> {
	let start = i6.find("nl('")?;
	let after = &i6[start + 4..];
	let end = after.find('\'')?;
	Some(after[..end].to_string())
}

pub fn api_imagedispatch(
	gid: &str,
	imgkey: &str,
	page: u32,
	mpvkey: &str,
	nl: Option<&str>,
	cookies: &str,
) -> Option<(String, Option<String>)> {
	let nl_val = nl.unwrap_or("");
	let body = format!(
		r#"{{"method":"imagedispatch","gid":{gid},"imgkey":"{imgkey}","page":{page},"mpvkey":"{mpvkey}","nl":"{nl_val}"}}"#
	);
	let mut resp = Request::post(get_api_url())
		.ok()?
		.header("Content-Type", "application/json")
		.header("Cookie", cookies)
		.header("User-Agent", USER_AGENT)
		.header("Referer", &get_base_url())
		.body(body.as_bytes())
		.send()
		.ok()?;

	let json: serde_json::Value = resp.get_json().ok()?;
	let img_url = json.get("i").and_then(|v| v.as_str())?.to_string();
	let nl_out = json
		.get("s")
		.and_then(|v| v.as_str())
		.filter(|s| !s.is_empty())
		.map(|s| s.to_string());

	Some((img_url, nl_out))
}

#[cfg(test)]
mod host_tests {
	use super::*;
	use aidoku_test::aidoku_test;

	#[aidoku_test]
	fn cookies_only_for_eh_hosts() {
		assert!(is_eh_host("https://e-hentai.org/g/1/abc/"));
		assert!(is_eh_host("https://exhentai.org/s/abc/1-1"));
		assert!(is_eh_host("https://api.e-hentai.org/api.php"));
		assert!(is_eh_host("https://E-Hentai.org:443/x"));
		// Hentai@Home image nodes are third-party servers
		assert!(!is_eh_host(
			"https://yawydzp.sonkpjwpofia.hath.network:10443/h/abc/x.jpg"
		));
		// substring tricks must not pass
		assert!(!is_eh_host("https://evil.example/?u=exhentai.org"));
		assert!(!is_eh_host("https://exhentai.org.evil.example/"));
		assert!(!is_eh_host("https://notexhentai.org/"));
		assert!(!is_eh_host("https://exhentai.org@evil.example/"));
		assert!(is_exhentai("https://exhentai.org/"));
		assert!(!is_exhentai("https://e-hentai.org/?r=exhentai.org"));
	}
}
