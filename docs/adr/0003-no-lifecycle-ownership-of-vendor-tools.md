# ADR 0003: No Lifecycle Ownership Of Vendor Tools

`af` may generate scripts and capture reports for vendor tools, but it does not
install, redistribute, license, or vendor-lock those tools.

Vendor backends are optional local-runner integrations, not default CI
dependencies.
