const REPO_ROOT = import.meta.dir.replace(/\/contracts\/erc20\/test$/, "");
export const MINICHAIN_BINARY = `${REPO_ROOT}/target/release/minichain`;

/**
 * Known bug: Block production does not include pending transactions.
 * See: https://github.com/jayendra-info/minichain/issues/12
 */
export const SKIP_BLOCK_PRODUCTION = false;

export async function runMinichain(...args: string[]): Promise<string> {
  const cmdStr = `${MINICHAIN_BINARY} ${args.join(" ")}`;
  const proc = Bun.spawn([MINICHAIN_BINARY, ...args]);
  const [exitCode, stdout, stderr] = await Promise.all([
    proc.exited,
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  
  if (exitCode !== 0) {
    throw new Error([
      `Command failed (exit ${exitCode})`,
      `cmd: ${cmdStr}`,
      `stdout: ${stdout || "(empty)"}`,
      `stderr: ${stderr || "(empty)"}`,
    ].join("\n"));
  }
  
  return stdout + stderr;
}

export async function initChain(dataDir: string): Promise<void> {
  await runMinichain("init", "--data-dir", dataDir, "--authorities", "1", "--force");
}

export async function createAccount(dataDir: string, name: string): Promise<string> {
  await runMinichain("account", "new", "--name", name, "--data-dir", dataDir);
  const keyFile = `${dataDir}/keys/${name}.json`;
  const keyContent = await Bun.file(keyFile).text();
  const keyJson = JSON.parse(keyContent);
  return keyJson.address;
}

export async function produceBlock(dataDir: string): Promise<void> {
  if (SKIP_BLOCK_PRODUCTION) {
    return;
  }
  await runMinichain("block", "produce", "--authority", "authority_0", "--data-dir", dataDir);
}

export async function getBalance(dataDir: string, address: string): Promise<number> {
  const output = await runMinichain("account", "balance", address, "--data-dir", dataDir);
  const match = output.match(/Balance:\s*(\d+)/);
  return match ? parseInt(match[1] ?? "0", 10) : 0;
}

export async function mintNative(
  dataDir: string,
  fromAuthority: string,
  toAddress: string,
  amount: number
): Promise<void> {
  await runMinichain(
    "account", "mint",
    "--from", fromAuthority,
    "--to", toAddress,
    "--amount", amount.toString(),
    "--data-dir", dataDir
  );
  await produceBlock(dataDir);
}
