"""Chapter abstraction for the Trahe ACME e2e demo.

Cuts the single-recording demo into coarse chapters so each one can be
reviewed and AI-narrated separately. Each chapter:

  - owns its own Playwright `BrowserContext` + `Page` + recorded video,
  - writes screenshots into its own subdirectory,
  - emits an `.srt` caption track (one cue per Act, timestamped from
    when the act was invoked),
  - emits a `.md` narration script (one bullet per Act, sourced from
    the act function's docstring — the input for AI TTS).

Designed to be used in `demo_trahe_acme.py::main()` like:

    chapters = build_chapters(args)
    state: DemoState = {}
    for ch in chapters:
        if ch.number not in selected:
            continue
        with ch.run(playwright, args) as session:
            page = session.page
            session.cue("Act 1: Wizard"); act_1_wizard(page)
            session.cue("Act 2: Files"); act_2_files(page)
            ...

The Chapter's `run()` context manager handles webm → mp4 conversion,
caption file generation, and screenshot index on exit.
"""

from __future__ import annotations

import contextlib
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Iterator, Optional


# ──────────────────────────────────────────────────────────────────────
# Caption / SRT helpers


def _format_srt_time(seconds: float) -> str:
    """SRT timestamp: HH:MM:SS,mmm — three-decimal millisecond precision."""
    seconds = max(0.0, seconds)
    h = int(seconds // 3600)
    m = int((seconds % 3600) // 60)
    s = int(seconds % 60)
    ms = int(round((seconds - int(seconds)) * 1000))
    if ms >= 1000:  # rounding edge case
        ms = 0
        s += 1
    return f"{h:02d}:{m:02d}:{s:02d},{ms:03d}"


@dataclass
class Cue:
    """One subtitle cue. `t` is seconds relative to the chapter's video start."""

    t: float
    title: str


# ──────────────────────────────────────────────────────────────────────
# Chapter definition + session


@dataclass(frozen=True)
class Chapter:
    """One demo chapter — number, slug, human title, list of (act_name,
    docstring) pairs the demo's `main()` will invoke inside `run()`."""

    number: int
    slug: str
    title: str
    # List of (function name, docstring) tuples — populated by
    # `build_chapters()` from the actual `demo_trahe_acme` module so
    # we can derive the narration script without re-importing.
    acts: tuple[tuple[str, str], ...] = field(default_factory=tuple)

    @property
    def dir_name(self) -> str:
        return f"{self.number:02d}-{self.slug}"


class ChapterSession:
    """Live recording handle: page + cue tracker + on-exit asset emitter."""

    def __init__(
        self,
        chapter: Chapter,
        chapter_dir: Path,
        page,
        context,
        playwright_browser,
        slowdown: float,
        repo_root: Path,
        no_video: bool,
        out_root: Path,
    ) -> None:
        self.chapter = chapter
        self.dir = chapter_dir
        self.shots_dir = chapter_dir / "screenshots"
        self.shots_dir.mkdir(parents=True, exist_ok=True)
        self.page = page
        self._context = context
        self._browser = playwright_browser
        self._slowdown = slowdown
        self._repo_root = repo_root
        self._no_video = no_video
        self._out_root = out_root
        self._start = time.monotonic()
        self._cues: list[Cue] = []
        self._shots: list[str] = []

    def cue(self, label: str) -> None:
        """Mark a cue point. Call before each Act invocation so the
        SRT timestamps line up with what's happening on screen."""
        self._cues.append(Cue(t=time.monotonic() - self._start, title=label))
        print(f"\n[{self.chapter.dir_name}] {label}")

    def screenshot(self, name: str) -> Path:
        """Chapter-scoped screenshot — lands in `<chapter>/screenshots/<name>.png`.

        Acts call this through the demo's existing `screenshot(page, name)`
        helper which we monkey-patch onto this session before the chapter
        runs (see `install_screenshot_hook()` in demo_trahe_acme.py).
        """
        path = self.shots_dir / f"{name}.png"
        self.page.screenshot(path=str(path), full_page=False)
        rel = path.relative_to(self._repo_root)
        print(f"  📸 {rel}")
        self._shots.append(path.name)
        return path

    def __enter__(self) -> "ChapterSession":
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        # Snapshot localStorage BEFORE closing the context so the next
        # chapter can resume the editor state. Without this each
        # chapter would start at the welcome page with no GLDF loaded.
        saved = _storage_state_path(self._out_root)
        try:
            saved.parent.mkdir(parents=True, exist_ok=True)
            self._context.storage_state(path=str(saved))
        except Exception as e:
            print(f"⚠ storage_state snapshot: {e}", file=sys.stderr)

        # Close the page first so the video flushes on the context, then
        # the context, then the browser.
        try:
            video_obj = self.page.video if not self._no_video else None
        except Exception:
            video_obj = None
        try:
            self._context.close()
        except Exception as e:
            print(f"⚠ context close: {e}", file=sys.stderr)
        try:
            self._browser.close()
        except Exception as e:
            print(f"⚠ browser close: {e}", file=sys.stderr)

        # Move the auto-generated webm into the chapter folder and slow-mo it.
        webm_target = self.dir / f"{self.chapter.slug}.webm"
        if video_obj is not None:
            try:
                raw = Path(video_obj.path())
                if raw.exists() and raw != webm_target:
                    shutil.move(str(raw), str(webm_target))
            except Exception as e:
                print(f"⚠ video move: {e}", file=sys.stderr)

        if webm_target.exists():
            kb = webm_target.stat().st_size // 1024
            print(f"  🎬 {webm_target.relative_to(self._repo_root)} ({kb} KB)")
            mp4 = self.dir / f"{self.chapter.slug}.mp4"
            self._slow_down(webm_target, mp4)

        self._write_srt()
        self._write_narration_md()
        self._write_screenshot_index()
        # Re-raise any in-chapter exception for the driver to decide on
        # fail-fast vs continue.
        return False

    def _slow_down(self, raw: Path, dst: Path) -> bool:
        if not shutil.which("ffmpeg"):
            print("⚠ ffmpeg not on PATH — skipping chapter mp4")
            return False
        cmd = [
            "ffmpeg", "-y", "-i", str(raw),
            "-filter:v", f"setpts={self._slowdown}*PTS",
            "-an", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "23",
            str(dst),
        ]
        print(f"  🎞  ffmpeg slo-mo ({self._slowdown}×) → {dst.relative_to(self._repo_root)}")
        r = subprocess.run(cmd, capture_output=True, text=True)
        if r.returncode != 0:
            print(r.stderr[-800:], file=sys.stderr)
            return False
        return True

    def _write_srt(self) -> None:
        """Emit an `.srt` with one cue per `cue()` call. Each cue runs from
        its own timestamp to the next cue's timestamp (or +5 s if last)."""
        if not self._cues:
            return
        # Apply the same slowdown factor used on the video so cue
        # timestamps line up with the .mp4 (not the source webm).
        scale = self._slowdown
        srt = self.dir / f"{self.chapter.slug}.srt"
        lines: list[str] = []
        for i, cue in enumerate(self._cues):
            start = cue.t * scale
            end = (
                self._cues[i + 1].t * scale if i + 1 < len(self._cues)
                else start + 5.0
            )
            lines.append(str(i + 1))
            lines.append(f"{_format_srt_time(start)} --> {_format_srt_time(end)}")
            lines.append(cue.title)
            lines.append("")
        srt.write_text("\n".join(lines), encoding="utf-8")
        print(f"  📝 {srt.relative_to(self._repo_root)} ({len(self._cues)} cues)")

    def _write_narration_md(self) -> None:
        """Emit a per-chapter narration brief. Each Act gets a bullet
        sourced from its docstring — that's what the LLM expands into
        voice-over prose."""
        md = self.dir / f"{self.chapter.slug}.md"
        out: list[str] = [
            f"# Chapter {self.chapter.number}: {self.chapter.title}",
            "",
            f"**Slug:** `{self.chapter.slug}` · "
            f"**MP4:** `{self.chapter.slug}.mp4` · "
            f"**Captions:** `{self.chapter.slug}.srt`",
            "",
            "## Acts in this chapter",
            "",
        ]
        for fn_name, doc in self.chapter.acts:
            first_para = (doc or "(no description)").strip().split("\n\n")[0]
            # Collapse newlines inside the first paragraph so the bullet
            # stays on one line for an LLM prompt.
            blurb = " ".join(line.strip() for line in first_para.splitlines() if line.strip())
            out.append(f"- **`{fn_name}`** — {blurb}")
        out.extend([
            "",
            "## Narration guidance",
            "",
            "Use the cues in the .srt as scene markers. Each Act bullet "
            "above is one paragraph of voice-over. Keep the tone calm "
            "and product-demo neutral; no exclamation marks.",
            "",
        ])
        md.write_text("\n".join(out), encoding="utf-8")
        print(f"  📜 {md.relative_to(self._repo_root)}")

    def _write_screenshot_index(self) -> None:
        if not self._shots:
            return
        idx = self.shots_dir / "index.md"
        out = [
            f"# Screenshots — Chapter {self.chapter.number}: {self.chapter.title}",
            "",
        ]
        for name in self._shots:
            out.append(f"- ![{name}]({name})")
        idx.write_text("\n".join(out) + "\n", encoding="utf-8")
        print(f"  🖼  {idx.relative_to(self._repo_root)} ({len(self._shots)} shots)")


# ──────────────────────────────────────────────────────────────────────
# Chapter manifest


# Slug + title + tuple of act-function names this chapter covers. The
# matching docstrings are pulled in by `build_chapters()` from the
# demo_trahe_acme module so we don't duplicate text.
CHAPTER_MANIFEST: tuple[tuple[int, str, str, tuple[str, ...]], ...] = (
    (
        1,
        "build-gldf",
        "Build the GLDF",
        (
            "act_1_wizard",
            "act_2_files",
            "act_3_light_sources",
            "act_4_control_gear",
            "act_5_equipment",
            "act_6_emitters",
            "act_7_variants_duplicate",
            "act_8_rebind_emitters_per_variant",
            "act_8c_rename_ids",
        ),
    ),
    (
        2,
        "metadata",
        "Fill the metadata panes",
        (
            "act_9_header_polish",
            "act_10_photometry",
            "act_11_electrical",
            "act_12_mechanical",
            "act_13_applications",
            "act_14_labels",
            "act_15_hyperlinks",
            "act_16_pictures",
            "act_17_emergency",
            "act_18_marketing",
            "act_19_custom_properties",
        ),
    ),
    (
        3,
        "3d-variants",
        "3D variants — edit + static showcase",
        (
            "act_20a_variant_mountings_view",
            "act_20_bevy",
        ),
    ),
    (
        4,
        "export-reimport",
        "Export, validate & re-import",
        (
            "act_21_export",
            "act_21b_xsd_validate",
            "act_22_reimport_walkthrough",
        ),
    ),
    (
        5,
        "tm33",
        "TM-33 round-trip & bootstrap",
        (
            "act_23_export_tm33",
            "act_24_reimport_tm33_to_files",
            "act_25_tm33_as_starter",
        ),
    ),
    (
        6,
        "lisp-bonus",
        "Bonus — Lisp REPL choreography",
        (
            "act_20b_lisp_repl",
        ),
    ),
)


def build_chapters(act_module) -> list[Chapter]:
    """Resolve the manifest into `Chapter` objects, pulling each act's
    docstring from `act_module` (typically the `demo_trahe_acme` module
    itself). This keeps the narration script in sync with the code."""
    out: list[Chapter] = []
    for number, slug, title, names in CHAPTER_MANIFEST:
        pairs: list[tuple[str, str]] = []
        for name in names:
            fn = getattr(act_module, name, None)
            doc = (fn.__doc__ or "") if fn is not None else ""
            pairs.append((name, doc))
        out.append(Chapter(number=number, slug=slug, title=title, acts=tuple(pairs)))
    return out


# ──────────────────────────────────────────────────────────────────────
# CLI helpers


def parse_chapter_selection(spec: Optional[str], total: int) -> set[int]:
    """Parse `--chapters 1,3-5` into a set of chapter numbers. None → all."""
    if spec is None or not spec.strip():
        return set(range(1, total + 1))
    selected: set[int] = set()
    for tok in spec.split(","):
        tok = tok.strip()
        if not tok:
            continue
        if "-" in tok:
            a, b = tok.split("-", 1)
            selected.update(range(int(a), int(b) + 1))
        else:
            selected.add(int(tok))
    return selected


# ──────────────────────────────────────────────────────────────────────
# Session opener


def _storage_state_path(out_root: Path) -> Path:
    """One shared JSON file at `<out_root>/chapters/_storage_state.json`
    keeps the editor's localStorage between chapters so chapter N+1 sees
    the GLDF chapter N built."""
    return out_root / "chapters" / "_storage_state.json"


@contextlib.contextmanager
def open_chapter(
    playwright,
    chapter: Chapter,
    out_root: Path,
    repo_root: Path,
    *,
    headless: bool,
    slow_mo: int,
    slowdown: float,
    no_video: bool,
    viewport_w: int = 1920,
    viewport_h: int = 1080,
    on_console_error: Optional[Callable[[str], None]] = None,
) -> Iterator[ChapterSession]:
    """Open a fresh BrowserContext for one chapter and yield a
    `ChapterSession`. On exit the context closes, the webm is moved into
    the chapter folder, the mp4 + .srt + .md + screenshot index are
    written.

    Carries the editor's `localStorage` between chapters via Playwright's
    `storage_state` snapshot — chapter N+1 opens with the editor showing
    whatever GLDF chapter N left in flight, which is what makes
    `--chapters 2-3` (or running just `3`) actually work."""
    chapter_dir = out_root / "chapters" / chapter.dir_name
    chapter_dir.mkdir(parents=True, exist_ok=True)
    # Wipe any prior screenshots so a re-run is clean.
    shots = chapter_dir / "screenshots"
    if shots.exists():
        for png in shots.glob("*.png"):
            png.unlink()

    # In headed mode, ask Chromium to open a window matching the
    # viewport (default 1920x1080). Without `--window-size` the OS
    # gives us a 1280x720 frame and the page content is letterboxed.
    launch_args: list[str] = []
    if not headless:
        launch_args.append(f"--window-size={viewport_w},{viewport_h}")
        launch_args.append("--window-position=0,0")
    browser = playwright.chromium.launch(
        headless=headless, slow_mo=slow_mo, args=launch_args
    )
    ctx_args: dict = {"viewport": {"width": viewport_w, "height": viewport_h}}
    if not no_video:
        ctx_args["record_video_dir"] = str(chapter_dir)
        ctx_args["record_video_size"] = {"width": viewport_w, "height": viewport_h}

    # Hand-off from the previous chapter: if a storage_state snapshot
    # exists, load it. Chapter 1 starts clean.
    saved = _storage_state_path(out_root)
    if saved.exists():
        ctx_args["storage_state"] = str(saved)
        print(f"  ↺ resumed editor state from {saved.relative_to(repo_root)}")

    context = browser.new_context(**ctx_args)
    page = context.new_page()
    if on_console_error is not None:
        page.on("console", lambda m: on_console_error(m.text) if m.type == "error" else None)

    session = ChapterSession(
        chapter=chapter,
        chapter_dir=chapter_dir,
        page=page,
        context=context,
        playwright_browser=browser,
        slowdown=slowdown,
        repo_root=repo_root,
        no_video=no_video,
        out_root=out_root,
    )
    try:
        yield session
    finally:
        session.__exit__(None, None, None)
