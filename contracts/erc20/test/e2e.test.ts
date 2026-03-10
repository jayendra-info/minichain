import { describe, it, beforeAll, afterAll, expect } from "bun:test";
import { tmpdir } from "os";
import { mkdir, rm } from "fs/promises";
import {
  initChain,
  createAccount,
  produceBlock,
  mintNative,
} from "./test-utils";
import { deploy, Contract, ERC20_ABI } from "./contract-client";

interface TestContext {
  dataDir: string;
  contractAddress: string;
}

let ctx: TestContext;
let asAlice!: Contract<typeof ERC20_ABI>;
let asBob!: Contract<typeof ERC20_ABI>;
let asCharlie!: Contract<typeof ERC20_ABI>;

beforeAll(async () => {
  const tempDir = `${tmpdir()}/minichain-erc20-test-${Date.now()}`;
  await mkdir(tempDir, { recursive: true });
  console.log(`\nTest data directory: ${tempDir}`);

  console.log("Initializing chain...");
  await initChain(tempDir);

  console.log("Creating accounts...");
  await createAccount(tempDir, "alice");
  await createAccount(tempDir, "bob");
  await createAccount(tempDir, "charlie");
  console.log("  alice, bob, charlie created");

  const aliceAddr = await Bun.file(`${tempDir}/keys/alice.json`).text().then(t => JSON.parse(t).address);
  console.log("Minting native tokens to alice...");
  await mintNative(tempDir, "authority_0", aliceAddr, 1000000);

  console.log("Deploying ERC20 contract...");
  const contractAddress = await deploy(tempDir, "alice");
  console.log(`  contract: ${contractAddress}`);

  console.log("Producing initial block...");
  await produceBlock(tempDir);

  ctx = { dataDir: tempDir, contractAddress };

  const erc20 = new Contract(tempDir, contractAddress, ERC20_ABI);
  asAlice   = erc20.connect("alice");
  asBob     = erc20.connect("bob");
  asCharlie = erc20.connect("charlie");
}, 30000);

afterAll(async () => {
  if (ctx?.dataDir) {
    console.log(`\nCleaning up: ${ctx.dataDir}`);
    await rm(ctx.dataDir, { recursive: true, force: true });
  }
});

describe("ERC20 Contract E2E Tests", () => {
  describe("Initial State", () => {
    it("should have zero ERC20 balances initially", async () => {
      const alice   = await asAlice.call("balanceOf", { address: 1 });
      const bob     = await asAlice.call("balanceOf", { address: 2 });
      const charlie = await asAlice.call("balanceOf", { address: 3 });

      expect(alice).toBe(0);
      expect(bob).toBe(0);
      expect(charlie).toBe(0);
    });
  });

  describe("Minting", () => {
    it("should mint tokens to alice (1000)", async () => {
      await asAlice.call("mint", { to: 1, amount: 1000 });

      const alice = await asAlice.call("balanceOf", { address: 1 });
      expect(alice).toBe(1000);
    });

    it("should increase total supply after minting", async () => {
      const alice   = await asAlice.call("balanceOf", { address: 1 });
      const bob     = await asAlice.call("balanceOf", { address: 2 });
      const charlie = await asAlice.call("balanceOf", { address: 3 });

      expect(alice + bob + charlie).toBe(1000);
    });
  });

  describe("Transfer", () => {
    it("should deduct from sender and credit receiver", async () => {
      await asAlice.call("transfer", { to: 2, amount: 300 });

      const alice = await asAlice.call("balanceOf", { address: 1 });
      const bob   = await asAlice.call("balanceOf", { address: 2 });

      expect(alice).toBe(700);
      expect(bob).toBe(300);
    });

    it("should preserve total supply after transfer", async () => {
      const alice   = await asAlice.call("balanceOf", { address: 1 });
      const bob     = await asAlice.call("balanceOf", { address: 2 });
      const charlie = await asAlice.call("balanceOf", { address: 3 });

      expect(alice + bob + charlie).toBe(1000);
    });
  });

  describe("Self-Transfer", () => {
    it("should not change balance on self-transfer", async () => {
      const before = await asAlice.call("balanceOf", { address: 1 });
      await asAlice.call("transfer", { to: 1, amount: 100 });
      const after = await asAlice.call("balanceOf", { address: 1 });

      expect(after).toBe(before);
    });
  });

  describe("Approval", () => {
    it("should approve spender", async () => {
      await asAlice.call("approve", { spender: 2, amount: 200 });

      const alice = await asAlice.call("balanceOf", { address: 1 });
      const bob   = await asAlice.call("balanceOf", { address: 2 });
      expect(alice).toBe(600);
      expect(bob).toBe(300);
    });
  });

  describe("TransferFrom", () => {
    it("should transfer on behalf of owner", async () => {
      await asBob.call("transferFrom", { from: 1, to: 3, amount: 150 });

      const alice   = await asAlice.call("balanceOf", { address: 1 });
      const charlie = await asAlice.call("balanceOf", { address: 3 });

      expect(alice).toBe(450);
      expect(charlie).toBe(150);
    });

    it("should preserve total supply after transferFrom", async () => {
      const alice   = await asAlice.call("balanceOf", { address: 1 });
      const bob     = await asAlice.call("balanceOf", { address: 2 });
      const charlie = await asAlice.call("balanceOf", { address: 3 });

      expect(alice + bob + charlie).toBe(1000);
    });
  });

  describe("Burn", () => {
    it("should burn tokens and reduce balance", async () => {
      const beforeAlice = await asAlice.call("balanceOf", { address: 1 });
      await asAlice.call("burn", { amount: 100 });
      const afterAlice = await asAlice.call("balanceOf", { address: 1 });

      expect(afterAlice).toBe(beforeAlice - 100);
    });

    it("should decrease total supply after burn", async () => {
      const alice   = await asAlice.call("balanceOf", { address: 1 });
      const bob     = await asAlice.call("balanceOf", { address: 2 });
      const charlie = await asAlice.call("balanceOf", { address: 3 });

      expect(alice + bob + charlie).toBe(900);
    });
  });

  describe("Multiple Transfers", () => {
    it("should handle sequential transfers", async () => {
      await asBob.call("transfer",     { to: 3, amount: 50 });
      await asCharlie.call("transfer", { to: 1, amount: 25 });

      const alice   = await asAlice.call("balanceOf", { address: 1 });
      const bob     = await asAlice.call("balanceOf", { address: 2 });
      const charlie = await asAlice.call("balanceOf", { address: 3 });

      expect(bob).toBe(200);
      expect(charlie).toBe(125);
      expect(alice).toBe(475);
    });
  });

  describe("Final State", () => {
    it("should have correct final balances", async () => {
      const alice   = await asAlice.call("balanceOf", { address: 1 });
      const bob     = await asAlice.call("balanceOf", { address: 2 });
      const charlie = await asAlice.call("balanceOf", { address: 3 });

      expect(alice).toBe(475);
      expect(bob).toBe(200);
      expect(charlie).toBe(125);
    });

    it("should have total supply equal to sum of balances", async () => {
      const alice   = await asAlice.call("balanceOf", { address: 1 });
      const bob     = await asAlice.call("balanceOf", { address: 2 });
      const charlie = await asAlice.call("balanceOf", { address: 3 });

      expect(alice + bob + charlie).toBe(800);
    });
  });
});
