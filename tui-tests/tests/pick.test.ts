import { test, expect } from "@microsoft/tui-test";
import { execSync } from "child_process";
import { mkdtempSync, writeFileSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";

const IX = process.env.IX_BIN ?? "ix";

/**
 * Set up a temporary git repo with the given files and a populated ix index.
 * Returns the repo directory path.
 */
function setupRepo(files: string[], session: string): string {
    const dir = mkdtempSync(join(tmpdir(), "ix-test-"));
    execSync("git init", { cwd: dir });
    execSync('git config user.email "t@t.com"', { cwd: dir });
    execSync('git config user.name "t"', { cwd: dir });
    for (const f of files) {
        writeFileSync(join(dir, f), "");
    }
    // Populate the index for this session
    execSync(`IX_SESSION=${session} ${IX} gs`, { cwd: dir });
    return dir;
}

test("pick: displays numbered slots", async ({ terminal }) => {
    const session = "tui-test-display";
    const dir = setupRepo(["main.rs", "lib.rs"], session);

    terminal.submit(`IX_SESSION=${session} ${IX} --pick`);

    await expect(terminal.getByText("[1]")).toBeVisible();
    await expect(terminal.getByText("[2]")).toBeVisible();
    await expect(terminal.getByText("main.rs")).toBeVisible();
    await expect(terminal.getByText("lib.rs")).toBeVisible();
});

test("pick: typing a number highlights the slot", async ({ terminal }) => {
    const session = "tui-test-highlight";
    const dir = setupRepo(["main.rs", "lib.rs", "foo.rs"], session);

    terminal.submit(`IX_SESSION=${session} ${IX} --pick`);
    await expect(terminal.getByText("[1]")).toBeVisible();

    terminal.type("2");

    // slot 2 should still be visible (highlighted)
    await expect(terminal.getByText("[2]")).toBeVisible();
});

test("pick: Enter confirms and outputs slot value", async ({ terminal }) => {
    const session = "tui-test-enter";
    const outFile = `/tmp/ix-pick-out-${session}`;
    const dir = setupRepo(["main.rs"], session);

    terminal.submit(
        `IX_SESSION=${session} ${IX} --pick > ${outFile}; echo "exit:$?"`
    );
    await expect(terminal.getByText("[1]")).toBeVisible();

    terminal.type("1");
    terminal.submit(""); // Enter

    await expect(terminal.getByText("exit:0")).toBeVisible();

    const out = execSync(`cat ${outFile}`).toString().trim();
    expect(out).toContain("main.rs");
});

test("pick: Escape aborts with no output", async ({ terminal }) => {
    const session = "tui-test-escape";
    const outFile = `/tmp/ix-pick-esc-${session}`;
    const dir = setupRepo(["main.rs"], session);

    terminal.submit(
        `IX_SESSION=${session} ${IX} --pick > ${outFile}; echo "exit:$?"`
    );
    await expect(terminal.getByText("[1]")).toBeVisible();

    terminal.keyPress("Escape");

    await expect(terminal.getByText("exit:1")).toBeVisible();
    const out = execSync(`cat ${outFile} 2>/dev/null || echo ""`).toString().trim();
    expect(out).toBe("");
});

test("pick: Ctrl+C aborts with exit 1", async ({ terminal }) => {
    const session = "tui-test-ctrlc";
    const outFile = `/tmp/ix-pick-ctrlc-${session}`;
    const dir = setupRepo(["main.rs"], session);

    terminal.submit(
        `IX_SESSION=${session} ${IX} --pick > ${outFile}; echo "exit:$?"`
    );
    await expect(terminal.getByText("[1]")).toBeVisible();

    terminal.keyPress("C-c");

    await expect(terminal.getByText("exit:1")).toBeVisible();
});

test("pick: shows message when no index exists", async ({ terminal }) => {
    terminal.submit(`IX_SESSION=no-such-session-99999 ${IX} --pick`);

    await expect(terminal.getByText(/no index found/i)).toBeVisible();
});

test("pick: range selection", async ({ terminal }) => {
    const session = "tui-test-range";
    const outFile = `/tmp/ix-pick-range-${session}`;
    const dir = setupRepo(["a.rs", "b.rs", "c.rs"], session);

    terminal.submit(
        `IX_SESSION=${session} ${IX} --pick > ${outFile}`
    );
    await expect(terminal.getByText("[1]")).toBeVisible();

    terminal.type("1-3");
    terminal.submit(""); // Enter

    await new Promise((r) => setTimeout(r, 200));
    const out = execSync(`cat ${outFile}`).toString();
    expect(out).toContain("a.rs");
    expect(out).toContain("b.rs");
    expect(out).toContain("c.rs");
});

test("pick: comma selection", async ({ terminal }) => {
    const session = "tui-test-comma";
    const outFile = `/tmp/ix-pick-comma-${session}`;
    const dir = setupRepo(["a.rs", "b.rs", "c.rs"], session);

    terminal.submit(
        `IX_SESSION=${session} ${IX} --pick > ${outFile}`
    );
    await expect(terminal.getByText("[1]")).toBeVisible();

    terminal.type("1,3");
    terminal.submit(""); // Enter

    await new Promise((r) => setTimeout(r, 200));
    const out = execSync(`cat ${outFile}`).toString();
    expect(out).toContain("a.rs");
    expect(out).not.toContain("b.rs");
    expect(out).toContain("c.rs");
});

test("pick: backspace removes characters", async ({ terminal }) => {
    const session = "tui-test-backspace";
    const dir = setupRepo(["a.rs", "b.rs"], session);

    terminal.submit(`IX_SESSION=${session} ${IX} --pick`);
    await expect(terminal.getByText("[1]")).toBeVisible();

    terminal.type("12");
    terminal.keyPress("Backspace");

    // Should now show "1" in the input field (the ">" prompt line)
    await expect(terminal.getByText(/> 1_/)).toBeVisible();
});
