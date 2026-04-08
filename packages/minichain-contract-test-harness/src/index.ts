import { mkdir } from "fs/promises";
import { tmpdir } from "os";

const HARNESS_ROOT_SUFFIX = "/packages/minichain-contract-test-harness/src";

export const REPO_ROOT = import.meta.dir.endsWith(HARNESS_ROOT_SUFFIX)
  ? import.meta.dir.slice(0, -HARNESS_ROOT_SUFFIX.length)
  : import.meta.dir.replace(/\/packages\/minichain-contract-test-harness\/src$/, "");

export const MINICHAIN_BINARY = `${REPO_ROOT}/target/release/minichain`;

export interface KeyFileJson {
  address: string;
  private_key: string;
  public_key: string;
}

export interface DeployContractArgs {
  dataDir: string;
  fromAlias: string;
  sourcePath: string;
  initData?: string;
  gasLimit?: number;
  autoProduceBlock?: boolean;
}

/**
 * Known bug: Block production does not include pending transactions.
 * See: https://github.com/jayendra-info/minichain/issues/12
 */
export const SKIP_BLOCK_PRODUCTION = false;

export function repoPath(...segments: string[]): string {
  return [REPO_ROOT, ...segments].join("/");
}

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

export async function createTempDataDir(prefix: string): Promise<string> {
  const dataDir = `${tmpdir()}/${prefix}-${Date.now()}`;
  await mkdir(dataDir, { recursive: true });
  return dataDir;
}

export async function initChain(dataDir: string): Promise<void> {
  await runMinichain("init", "--data-dir", dataDir, "--authorities", "1", "--force");
}

export async function loadKeyFile(dataDir: string, name: string): Promise<KeyFileJson> {
  return await Bun.file(`${dataDir}/keys/${name}.json`).json() as KeyFileJson;
}

export async function loadAddress(dataDir: string, name: string): Promise<string> {
  const { address } = await loadKeyFile(dataDir, name);
  return address;
}

export async function createAccount(dataDir: string, name: string): Promise<string> {
  await runMinichain("account", "new", "--name", name, "--data-dir", dataDir);
  return await loadAddress(dataDir, name);
}

export async function produceBlock(dataDir: string): Promise<void> {
  if (SKIP_BLOCK_PRODUCTION) {
    return;
  }
  await runMinichain("block", "produce", "--authority", "@authority_0", "--data-dir", dataDir);
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
  amount: number,
): Promise<void> {
  await runMinichain(
    "account",
    "mint",
    "--from",
    `@${fromAuthority}`,
    "--to",
    toAddress,
    "--amount",
    amount.toString(),
    "--data-dir",
    dataDir,
  );
  await produceBlock(dataDir);
}

export async function deployContract(args: DeployContractArgs): Promise<string> {
  const output = await runMinichain(
    "deploy",
    "--from",
    `@${args.fromAlias}`,
    "--source",
    args.sourcePath,
    "--gas-limit",
    (args.gasLimit ?? 400000).toString(),
    "--data-dir",
    args.dataDir,
    ...(args.initData ? ["--init-data", args.initData] : []),
  );
  const match = output.match(/Contract Address:\s*(0x[0-9a-f]+)/i);
  if (!match) {
    throw new Error(`Failed to parse contract address: ${output}`);
  }

  if (args.autoProduceBlock ?? true) {
    await produceBlock(args.dataDir);
  }

  return match[1]!;
}
