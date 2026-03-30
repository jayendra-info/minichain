import {
  deployContract,
  produceBlock,
  runMinichain,
} from "@minichain/contract-test-harness";

const AUCTION_CONTRACT_PATH = (() => {
  const base = import.meta.dir.replace(/\/test$/, "");
  return `${base}/src/auction.asm`;
})();

export class AuctionClient {
  constructor(
    private readonly dataDir: string,
    private readonly address: string,
    private readonly callerAlias: string,
  ) {}

  async bid(amount: number): Promise<void> {
    await this.send(amount);
  }

  async initialize(): Promise<void> {
    await this.send(0);
  }

  async withdraw(): Promise<void> {
    await this.send(0);
  }

  private async send(amount: number): Promise<void> {
    await runMinichain(
      "call",
      "--from",
      `@${this.callerAlias}`,
      "--to",
      this.address,
      "--amount",
      amount.toString(),
      "--data-dir",
      this.dataDir,
      "--gas-limit",
      "250000",
    );
    await produceBlock(this.dataDir);
  }
}

export async function deployAuction(dataDir: string, sellerAlias: string): Promise<string> {
  return await deployContract({
    dataDir,
    fromAlias: sellerAlias,
    sourcePath: AUCTION_CONTRACT_PATH,
    gasLimit: 500000,
  });
}
