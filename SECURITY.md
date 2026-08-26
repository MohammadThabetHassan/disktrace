# Security policy

DiskTrace processes untrusted disk-image bytes and can write recovered artifacts to a local destination. Parser bounds, source integrity, destination safety, fixture controls, and the local verification matrix are security-relevant parts of the project.

## Supported versions

The project is pre-release. Security fixes are evaluated against the current default branch after the repository is published. No historical release branch is supported yet.

| Version line | Security support |
|---|---|
| Current unreleased workspace | Evaluated before public publication. |
| Future tagged current release | Intended support target after a public release process and maintainer contact are established. |
| Superseded future releases | Not yet defined. |

## Reporting a vulnerability

Use GitHub’s [private vulnerability-reporting channel](https://github.com/MohammadThabetHassan/disktrace/security/advisories/new) for confidential DiskTrace reports. Do **not** post sensitive findings, exploit images, recovered personal data, credentials, or proof-of-concept samples in public issues, pull requests, discussions, or comments.

If GitHub’s private reporting form is temporarily unavailable, do not disclose the finding publicly. Use the maintainer contact listed in the repository profile only if it is available and explicitly designated for confidential reports; include `DiskTrace security report` in the subject line and remove private bytes and personally identifying information.

A useful report includes the affected revision, operating system, minimal synthetic reproduction steps, expected and observed behavior, candidate method, relevant parser or workflow boundary, and potential impact. Please remove private bytes and personally identifying information before sharing any material.

## Scope examples

Security-relevant reports may include:

- Out-of-bounds parsing, integer overflow, or unbounded allocation caused by image bytes.
- A source-integrity, destination-policy, receipt, or saved-session bypass.
- A recovery path that writes outside the approved destination.
- Unsafe preview, rendering, or execution of recovered content.
- Unexpected network transmission, telemetry, sensitive-data exposure, or dependency compromise.
- A method explanation that materially overstates what the code validates and could lead users to unsafe reliance.

Ordinary feature requests, unsupported filesystem requests, and expected refusals of malformed images should be reported through the future public issue tracker once available.

## Disclosure expectations

Please give maintainers a reasonable opportunity to reproduce and prepare a fix before public disclosure. Maintainers should acknowledge a valid report, establish a safe communication channel, provide status updates when practical, and coordinate public disclosure only after a fix or documented mitigation is available.

DiskTrace must not request or retain real source images to validate a report. A minimized benign fixture, a redacted byte layout, or a deterministic generator is preferred.

## Operational safety

Do not run a recovered exploit sample or document with active content to test a report. Use an isolated analysis environment and the project’s deterministic fixture style wherever possible. The security policy does not replace organizational incident-response, legal, or evidence-handling procedures.
