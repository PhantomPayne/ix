import { test, expect } from "@microsoft/tui-test";
import { execSync } from "child_process";
import { mkdtempSync, writeFileSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";

const IX = process.env.IX_BIN ?? "ix";

function setupRepo(files: string[], session: string): string {
    const dir = mkdtempSync(join(tmpdir(), "ix-test-"));
    execSync("git init", { cwd: dir });
    execSync('git config user.email "t@t.com"', { cwd: dir });
    execSync('git config user.name "t"', { cwd: dir });
    for (const f of files) {
        writeFileSync(join(dir, f), "");
    }
    return dir;
}

test("display: gs shows untracked files with slot numbers", async ({ terminal }) => {
    const session = "tui-test-disp-gs";
    const dir = setupRepo(["foo.rs", "bar.rs"], session);

    terminal.submit(`IX_SESSION=${session} ${IX} gs`);

    await expect(terminal.getByText("[1]")).toBeVisible();
    await expect(terminal.getByText("[2]")).toBeVisible();
    await expect(terminal.getByText(/foo\.rs|bar\.rs/)).toBeVisible();
});

test("display: pick shows provider name and item count", async ({ terminal }) => {
    const session = "tui-test-disp-count";
    const dir = setupRepo(["a.rs", "b.rs", "c.rs"], session);
    execSync(`IX_SESSION=${session} ${IX} gs`, { cwd: dir });

    terminal.submit(`IX_SESSION=${session} ${IX} --pick`);

    // Title bar should include item count
    await expect(terminal.getByText(/3 items/)).toBeVisible();
});

test("display: pick input prompt shows > prefix", async ({ terminal }) => {
    const session = "tui-test-disp-prompt";
    const dir = setupRepo(["main.rs"], session);
    execSync(`IX_SESSION=${session} ${IX} gs`, { cwd: dir });

    terminal.submit(`IX_SESSION=${session} ${IX} --pick`);
    await expect(terminal.getByText("[1]")).toBeVisible();

    // Input prompt is visible at bottom
    await expect(terminal.getByText(/> _/)).toBeVisible();
});

test("display: pick shows help text at bottom", async ({ terminal }) => {
    const session = "tui-test-disp-help";
    const dir = setupRepo(["main.rs"], session);
    execSync(`IX_SESSION=${session} ${IX} gs`, { cwd: dir });

    terminal.submit(`IX_SESSION=${session} ${IX} --pick`);
    await expect(terminal.getByText("[1]")).toBeVisible();

    await expect(terminal.getByText(/Enter.*confirm/i)).toBeVisible();
    await expect(terminal.getByText(/Esc.*cancel/i)).toBeVisible();
});
