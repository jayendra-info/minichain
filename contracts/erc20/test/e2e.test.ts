import { afterAll, beforeAll, describe, expect, it } from "bun:test";
import { rm } from "fs/promises";
import {
  createAccount,
  createTempDataDir,
  initChain,
  loadAddress,
  mintNative,
} from "@minichain/contract-test-harness";
import { ContractClient, deployErc20, SELECTORS } from "./contract-client";

interface TestContext {
  dataDir: string;
  contractAddress: string;
  ids: Record<string, bigint>;
}

let ctx: TestContext;
let asAlice: ContractClient;
let asBob: ContractClient;
let asCharlie: ContractClient;

beforeAll(async () => {
  const dataDir = await createTempDataDir("minichain-erc20-test");

  await initChain(dataDir);
  await createAccount(dataDir, "alice");
  await createAccount(dataDir, "bob");
  await createAccount(dataDir, "charlie");

  const aliceAddress = await loadAddress(dataDir, "alice");
  const bobAddress = await loadAddress(dataDir, "bob");
  const charlieAddress = await loadAddress(dataDir, "charlie");
  await mintNative(dataDir, "authority_0", aliceAddress, 1_000_000);
  await mintNative(dataDir, "authority_0", bobAddress, 500_000);
  await mintNative(dataDir, "authority_0", charlieAddress, 500_000);

  const deployed = await deployErc20(dataDir, "alice", {
    name: "MiniCoin",
    symbol: "MINI",
    decimals: 18,
    initialSupply: 0,
  });

  ctx = {
    dataDir,
    contractAddress: deployed.address,
    ids: deployed.ids,
  };

  asAlice = new ContractClient(dataDir, deployed.address, "alice");
  asBob = new ContractClient(dataDir, deployed.address, "bob");
  asCharlie = new ContractClient(dataDir, deployed.address, "charlie");
}, 30000);

afterAll(async () => {
  if (ctx?.dataDir) {
    await rm(ctx.dataDir, { recursive: true, force: true });
  }
});

describe("ERC20 Contract E2E Tests", () => {
  it("returns deployed metadata", async () => {
    expect(await asAlice.queryString(SELECTORS.name)).toBe("MiniCoin");
    expect(await asAlice.queryString(SELECTORS.symbol)).toBe("MINI");
    expect(await asAlice.queryU64(SELECTORS.decimals)).toBe(18);
  });

  it("starts with zero balances and zero total supply", async () => {
    expect(await asAlice.queryU64(SELECTORS.totalSupply)).toBe(0);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.alice])).toBe(0);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.bob])).toBe(0);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.charlie])).toBe(0);
  });

  it("mints tokens to alice", async () => {
    await asAlice.send(SELECTORS.mint, [ctx.ids.alice, 1000]);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.alice])).toBe(1000);
    expect(await asAlice.queryU64(SELECTORS.totalSupply)).toBe(1000);
  });

  it("rejects mint attempts from non-owner", async () => {
    await asBob.send(SELECTORS.mint, [ctx.ids.bob, 500]);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.bob])).toBe(0);
    expect(await asAlice.queryU64(SELECTORS.totalSupply)).toBe(1000);
  });

  it("transfers from alice to bob", async () => {
    await asAlice.send(SELECTORS.transfer, [ctx.ids.bob, 300]);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.alice])).toBe(700);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.bob])).toBe(300);
    expect(await asAlice.queryU64(SELECTORS.totalSupply)).toBe(1000);
  });

  it("keeps balance unchanged on self-transfer", async () => {
    const before = await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.alice]);
    await asAlice.send(SELECTORS.transfer, [ctx.ids.alice, 100]);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.alice])).toBe(before);
  });

  it("tracks approvals and remaining allowance", async () => {
    await asAlice.send(SELECTORS.approve, [ctx.ids.bob, 200]);
    expect(await asAlice.queryU64(SELECTORS.allowance, [ctx.ids.alice, ctx.ids.bob])).toBe(200);

    await asBob.send(SELECTORS.transferFrom, [ctx.ids.alice, ctx.ids.charlie, 150]);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.alice])).toBe(550);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.charlie])).toBe(150);
    expect(await asAlice.queryU64(SELECTORS.allowance, [ctx.ids.alice, ctx.ids.bob])).toBe(50);
    expect(await asAlice.queryU64(SELECTORS.totalSupply)).toBe(1000);
  });

  it("burns tokens and reduces total supply", async () => {
    await asAlice.send(SELECTORS.burn, [100]);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.alice])).toBe(450);
    expect(await asAlice.queryU64(SELECTORS.totalSupply)).toBe(900);
  });

  it("handles additional transfers after burn", async () => {
    await asBob.send(SELECTORS.transfer, [ctx.ids.charlie, 50]);
    await asCharlie.send(SELECTORS.transfer, [ctx.ids.alice, 25]);

    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.alice])).toBe(475);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.bob])).toBe(250);
    expect(await asAlice.queryU64(SELECTORS.balanceOf, [ctx.ids.charlie])).toBe(175);
    expect(await asAlice.queryU64(SELECTORS.totalSupply)).toBe(900);
  });
});
