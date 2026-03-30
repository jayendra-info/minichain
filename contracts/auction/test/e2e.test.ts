import { afterAll, beforeAll, describe, expect, it } from "bun:test";
import { rm } from "fs/promises";
import {
  createAccount,
  createTempDataDir,
  getBalance,
  initChain,
  loadAddress,
  mintNative,
} from "@minichain/contract-test-harness";
import { AuctionClient, deployAuction } from "./auction-client";

interface TestContext {
  dataDir: string;
  contractAddress: string;
}

let ctx: TestContext;
let asBob: AuctionClient;
let asCharlie: AuctionClient;

beforeAll(async () => {
  const dataDir = await createTempDataDir("minichain-auction-test");

  await initChain(dataDir);
  await createAccount(dataDir, "alice");
  await createAccount(dataDir, "bob");
  await createAccount(dataDir, "charlie");

  await mintNative(dataDir, "authority_0", await loadAddress(dataDir, "alice"), 1_000_000);
  await mintNative(dataDir, "authority_0", await loadAddress(dataDir, "bob"), 500_000);
  await mintNative(dataDir, "authority_0", await loadAddress(dataDir, "charlie"), 500_000);

  const contractAddress = await deployAuction(dataDir, "alice");

  ctx = {
    dataDir,
    contractAddress,
  };

  asBob = new AuctionClient(dataDir, contractAddress, "bob");
  asCharlie = new AuctionClient(dataDir, contractAddress, "charlie");
}, 30000);

afterAll(async () => {
  if (ctx?.dataDir) {
    await rm(ctx.dataDir, { recursive: true, force: true });
  }
});

describe("Auction Contract E2E Tests", () => {
  it("starts with zero contract balance", async () => {
    expect(await getBalance(ctx.dataDir, ctx.contractAddress)).toBe(0);
  });

  it("credits the first value-bearing call into the contract balance", async () => {
    await asBob.bid(50);
    expect(await getBalance(ctx.dataDir, ctx.contractAddress)).toBe(50);
  });

  it("accepts a later higher bid and adds more native funds", async () => {
    const before = await getBalance(ctx.dataDir, await loadAddress(ctx.dataDir, "bob"));
    await asBob.bid(150);
    const after = await getBalance(ctx.dataDir, await loadAddress(ctx.dataDir, "bob"));

    expect(after).toBeLessThan(before);
    expect(await getBalance(ctx.dataDir, ctx.contractAddress)).toBe(200);
  });

  it("accepts a higher outbid and accumulates native funds in the contract", async () => {
    const before = await getBalance(ctx.dataDir, await loadAddress(ctx.dataDir, "charlie"));
    await asCharlie.bid(200);
    const after = await getBalance(ctx.dataDir, await loadAddress(ctx.dataDir, "charlie"));

    expect(after).toBeLessThan(before);
    expect(await getBalance(ctx.dataDir, ctx.contractAddress)).toBe(400);
  });

  it("currently leaves contract balance unchanged on refund withdrawal", async () => {
    const before = await getBalance(ctx.dataDir, ctx.contractAddress);
    await asBob.withdraw();
    expect(await getBalance(ctx.dataDir, ctx.contractAddress)).toBe(before);
  });
});
