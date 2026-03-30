import { produceBlock, runMinichain } from "@minichain/contract-test-harness";

const ERC20_CONTRACT_PATH = (() => {
  const base = import.meta.dir.replace(/\/test$/, "");
  return `${base}/src/erc20.asm`;
})();

type Arg = number | bigint;
type AddressMap = Record<string, string>;
type AddressIdMap = Record<string, bigint>;

function encodeWord(value: Arg): string {
  const buffer = new ArrayBuffer(8);
  const view = new DataView(buffer);
  view.setBigUint64(0, BigInt(value), true);
  return Buffer.from(buffer).toString("hex");
}

function encodeCall(selector: number, args: readonly Arg[]): string {
  return [encodeWord(selector), ...args.map(encodeWord)].join("");
}

function decodeU64(hexOutput: string): number {
  const match = hexOutput.match(/Result:\s*0x([0-9a-f]+)/i);
  if (!match) return 0;
  const bytes = Buffer.from(match[1]!, "hex");
  const padded = Buffer.concat([bytes, Buffer.alloc(Math.max(0, 8 - bytes.length))]).subarray(0, 8);
  return Number(padded.readBigUInt64LE(0));
}

function decodeString(hexOutput: string): string {
  const match = hexOutput.match(/Result:\s*0x([0-9a-f]+)/i);
  if (!match) return "";
  return Buffer.from(match[1]!, "hex").toString("ascii").replace(/\0+$/, "");
}

function encodeAscii8(value: string): bigint {
  const buffer = Buffer.alloc(8);
  buffer.write(value.slice(0, 8), 0, "ascii");
  return buffer.readBigUInt64LE(0);
}

export function addressToId(address: string): bigint {
  const bytes = Buffer.from(address.replace(/^0x/i, ""), "hex");
  return bytes.readBigUInt64LE(0);
}

async function keyAddress(dataDir: string, alias: string): Promise<string> {
  const keyJson = await Bun.file(`${dataDir}/keys/${alias}.json`).json() as { address: string };
  return keyJson.address;
}

export const SELECTORS = {
  totalSupply: 0x00,
  balanceOf: 0x01,
  transfer: 0x02,
  approve: 0x03,
  transferFrom: 0x04,
  allowance: 0x05,
  mint: 0x06,
  burn: 0x07,
  name: 0x08,
  symbol: 0x09,
  decimals: 0x0a,
  init: 0xff,
} as const;

export class ContractClient {
  constructor(
    private readonly dataDir: string,
    private readonly address: string,
    private readonly callerAlias: string,
  ) {}

  private callerRef(): string {
    return `@${this.callerAlias}`;
  }

  async queryU64(selector: number, args: readonly Arg[] = []): Promise<number> {
    const output = await runMinichain(
      "call",
      "--query",
      "--from", this.callerRef(),
      "--to", this.address,
      "--data", encodeCall(selector, args),
      "--data-dir", this.dataDir,
    );
    return decodeU64(output);
  }

  async queryString(selector: number): Promise<string> {
    const output = await runMinichain(
      "call",
      "--query",
      "--from", this.callerRef(),
      "--to", this.address,
      "--data", encodeCall(selector, []),
      "--data-dir", this.dataDir,
    );
    return decodeString(output);
  }

  async send(selector: number, args: readonly Arg[] = []): Promise<void> {
    await runMinichain(
      "call",
      "--from", this.callerRef(),
      "--to", this.address,
      "--data", encodeCall(selector, args),
      "--data-dir", this.dataDir,
      "--gas-limit", "250000",
    );
    await produceBlock(this.dataDir);
  }
}

export async function deployErc20(
  dataDir: string,
  ownerAlias: string,
  metadata: { name: string; symbol: string; decimals: number; initialSupply?: number; initialRecipientAlias?: string },
): Promise<{ address: string; ids: AddressIdMap; addresses: AddressMap }> {
  const addresses = {
    alice: await keyAddress(dataDir, "alice"),
    bob: await keyAddress(dataDir, "bob"),
    charlie: await keyAddress(dataDir, "charlie"),
  };
  const ids = Object.fromEntries(
    Object.entries(addresses).map(([alias, address]) => [alias, addressToId(address)]),
  ) as AddressIdMap;
  const initialRecipientAlias = metadata.initialRecipientAlias ?? ownerAlias;
  const initData = encodeCall(SELECTORS.init, [
    ids[ownerAlias]!,
    encodeAscii8(metadata.name),
    encodeAscii8(metadata.symbol),
    metadata.decimals,
    ids[initialRecipientAlias]!,
    metadata.initialSupply ?? 0,
  ]);

  const output = await runMinichain(
    "deploy",
    "--from", `@${ownerAlias}`,
    "--source", ERC20_CONTRACT_PATH,
    "--init-data", initData,
    "--gas-limit", "400000",
    "--data-dir", dataDir,
  );
  const match = output.match(/Contract Address:\s*(0x[0-9a-f]+)/i);
  if (!match) {
    throw new Error(`Failed to parse contract address: ${output}`);
  }

  await produceBlock(dataDir);

  return { address: match[1]!, ids, addresses };
}
