# `wiki/` — source of truth for the GitHub Wiki

These Markdown files are the **canonical source** for the LiteForge GitHub Wiki at
<https://github.com/seanpoyner/liteforge/wiki>.

**Do not edit the wiki through the GitHub web UI** — changes there will be overwritten on the next
sync. Edit the files here, commit them with the rest of the repo (so they get PR review and live in
history), then publish:

```bash
scripts/sync-wiki.sh
```

## Conventions

- **One page per file.** The filename (minus `.md`) is the wiki page slug. `Tools-and-Agents.md`
  becomes the page **“Tools and Agents”** at `/wiki/Tools-and-Agents`.
- **Special pages:** `Home.md` (landing page), `_Sidebar.md` (left nav), `_Footer.md` (footer).
- **Internal links** use the slug without extension: `[Quickstart](Quickstart)`.
- **Diagrams** are [Mermaid](https://mermaid.js.org/) fenced blocks — GitHub renders them natively
  in the wiki, so there is no image/export step.
- This wiki is a **guided tour**. Deep API reference lives on [docs.rs](https://docs.rs/liteforge)
  and in the repo's [`docs/`](../docs) tree — link out rather than duplicating it here.

See [Contributing](Contributing.md) for the publishing workflow and the one‑time wiki init step.
