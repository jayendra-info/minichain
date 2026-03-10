import { runMinichain, produceBlock } from "./test-utils";

const ERC20_CONTRACT_PATH = (() => {
  const base = import.meta.dir.replace(/\/test$/, "");
  return `${base}/erc20.asm`;
})();

// ─── ABI type layer ───────────────────────────────────────────────────────────

type AbiParamType = "uint64";
type AbiOutput = AbiParamType | null;

interface AbiParam {
  name: string;
  type: AbiParamType;
}

interface AbiFunction {
  selector: number;
  inputs: readonly AbiParam[];
  output: AbiOutput;
}

type Abi = Record<string, AbiFunction>;

// Named argument object derived from the inputs tuple: { to: number; amount: number }
type ArgsFromInputs<T extends readonly AbiParam[]> = {
  [P in T[number] as P["name"]]: number;
};

type ResultFromOutput<T extends AbiOutput> = T extends null ? void : number;

// ─── ERC20 ABI definition ─────────────────────────────────────────────────────

export const ERC20_ABI = {
  totalSupply:  { selector: 0x00, inputs: [],                                   output: "uint64" },
  balanceOf:    { selector: 0x01, inputs: [{ name: "address", type: "uint64" }], output: "uint64" },
  transfer:     { selector: 0x02, inputs: [{ name: "to",      type: "uint64" },
                                           { name: "amount",  type: "uint64" }], output: null },
  approve:      { selector: 0x03, inputs: [{ name: "spender", type: "uint64" },
                                           { name: "amount",  type: "uint64" }], output: null },
  transferFrom: { selector: 0x04, inputs: [{ name: "from",    type: "uint64" },
                                           { name: "to",      type: "uint64" },
                                           { name: "amount",  type: "uint64" }], output: null },
  allowance:    { selector: 0x05, inputs: [{ name: "owner",   type: "uint64" },
                                           { name: "spender", type: "uint64" }], output: "uint64" },
  mint:         { selector: 0x06, inputs: [{ name: "to",      type: "uint64" },
                                           { name: "amount",  type: "uint64" }], output: null },
  burn:         { selector: 0x07, inputs: [{ name: "amount",  type: "uint64" }], output: null },
  name:         { selector: 0x08, inputs: [],                                   output: "uint64" },
  symbol:       { selector: 0x09, inputs: [],                                   output: "uint64" },
  decimals:     { selector: 0x0a, inputs: [],                                   output: "uint64" },
} as const satisfies Abi;

// ─── Encoding / decoding ──────────────────────────────────────────────────────

function encodeCalldata(
  selector: number,
  inputs: readonly AbiParam[],
  args: Record<string, number>
): string {
  const enc = (v: number) => v.toString(16).padStart(16, "0");
  return (
    selector.toString(16).padStart(2, "0") +
    inputs.map((p) => enc(args[p.name] ?? 0)).join("")
  );
}

function parseResult(output: string): number {
  const match = output.match(/Result:\s*0x([0-9a-f]+)/i);
  return match ? parseInt(match[1] ?? "0", 16) : 0;
}

// ─── Contract class ───────────────────────────────────────────────────────────

export class Contract<A extends Abi> {
  constructor(
    private readonly dataDir: string,
    private readonly address: string,
    private readonly abi: A,
    private readonly caller?: string
  ) {}

  /** Returns a new Contract instance bound to the given caller name. */
  connect(caller: string): Contract<A> {
    return new Contract(this.dataDir, this.address, this.abi, caller);
  }

  async call<K extends keyof A & string>(
    fn: K,
    args: ArgsFromInputs<A[K]["inputs"]>
  ): Promise<ResultFromOutput<A[K]["output"]>> {
    if (!this.caller) throw new Error("No caller set — use .connect(callerName) first");
    const { selector, inputs, output } = this.abi[fn] as AbiFunction;
    const data = encodeCalldata(selector, inputs, args as Record<string, number>);
    const raw = await runMinichain(
      "call",
      "--from", this.caller,
      "--to", this.address,
      "--data", data,
      "--data-dir", this.dataDir
    );
    await produceBlock(this.dataDir);
    return (output !== null ? parseResult(raw) : undefined) as ResultFromOutput<A[K]["output"]>;
  }
}

// ─── deploy ───────────────────────────────────────────────────────────────────

export async function deploy(dataDir: string, fromName: string): Promise<string> {
  const output = await runMinichain(
    "deploy",
    "--from", fromName,
    "--source", ERC20_CONTRACT_PATH,
    "--gas-limit", "250000",
    "--data-dir", dataDir
  );
  const match = output.match(/Contract Address:\s*(0x[0-9a-f]+)/i);
  if (!match) throw new Error(`Failed to parse contract address: ${output}`);
  return match[1]!;
}
