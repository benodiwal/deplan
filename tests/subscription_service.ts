import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SubscriptionService } from "../target/types/subscription_service";

describe("subscription_service", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.SubscriptionService as Program<SubscriptionService>;

  it("Is initialized!", async () => {
  });
});
