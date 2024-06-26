import { test, assert } from "vitest";
import { dirname } from "path";
import { fileURLToPath } from "url";

import { runScenario, dhtSync } from "@holochain/tryorama";
import { AppWebsocket, Record } from "@holochain/client";
import { EntryRecord } from "@holochain-open-dev/utils";

test("setup provider, sender and recipient, and send an email", async (t) => {
  await runScenario(
    async (scenario) => {
      const testHappUrl =
        dirname(fileURLToPath(import.meta.url)) + "/../fixture/fixture.happ";
      const providerHappUrl =
        dirname(fileURLToPath(import.meta.url)) +
        "/../fixture/email_notifications_provider-debug.happ";

      // Add 2 players with the test hApp to the Scenario. The returned players
      // can be destructured.
      const [provider, sender, recipient] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: providerHappUrl } },
        { appBundleSource: { path: testHappUrl } },
        { appBundleSource: { path: testHappUrl } },
      ]);

      const providerServiceCell = provider.namedCells.get(
        "email_notifications_bridge",
      );
      const providerProviderCell = provider.namedCells.get(
        "email_notifications_provider",
      );
      assert.equal(
        sender.namedCells
          .get("email_notifications_service")
          .cell_id[0].toString(),
        providerServiceCell.cell_id[0].toString(),
      );

      // Shortcut peer discovery through gossip and register all agents in every
      // conductor of the scenario.
      await scenario.shareAllAgents();

      /* Setup provider */

      // Publish Email Credentials
      const emailCredentials = {
        sender_email_address: "some@address.com",
        password: "some@address.com",
        smtp_relay_url: "smtp.gmail.com",
      };
      await providerProviderCell.callZome({
        zome_name: "email_notifications_provider",
        fn_name: "publish_new_email_credentials",
        payload: emailCredentials,
      });

      await dhtSync(
        [sender, recipient],
        sender.namedCells.get("fixture_dna").cell_id[0],
      );

      const NOTIFICATION_TYPE = "some_app_defined_notification_type";

      const emailAddress = "some@address.com";
      // Register email address
      await recipient.namedCells.get("fixture_dna").callZome({
        zome_name: "notifications",
        fn_name: "set_notifications_settings",
        payload: {
          settings_by_notification_type: {
            [NOTIFICATION_TYPE]: {
              enabled: true,
              providers: ["work_email"],
            },
          },
          available_notification_providers: {
            work_email: {
              type: "Email",
              email_address: emailAddress,
            },
          },
        },
      });

      await dhtSync(
        [sender, recipient],
        sender.namedCells.get("fixture_dna").cell_id[0],
      );

      /* Send email notification */

      const email = {
        subject: "Some important email message",
        body: "Lorem ipsum blabla",
      };

      await Promise.race([
        new Promise((_, reject) => setTimeout(() => reject(), 20000)),
        new Promise(async (resolve) => {
          // FCM provider zome sends signal
          (provider.appWs as AppWebsocket).on("signal", (signal) => {
            console.log(signal);
            const payload = signal.payload as any;
            assert.deepEqual(payload.email_address, emailAddress);
            assert.deepEqual(payload.email, email);
            assert.deepEqual(payload.credentials, emailCredentials);
            resolve(undefined);
          });
          // Get notification settings from fixture notification settings zome
          const settings: Record = await sender.namedCells
            .get("fixture_dna")
            .callZome({
              zome_name: "notifications",
              fn_name: "get_notifications_settings_for",
              payload: recipient.agentPubKey,
            });

          // Send email
          const recipientNotificationsSettings = new EntryRecord<any>(settings)
            .entry;
          const selectedProvider =
            recipientNotificationsSettings.settings_by_notification_type[
              NOTIFICATION_TYPE
            ].providers[0];
          const email_address =
            recipientNotificationsSettings.available_notification_providers[
              selectedProvider
            ].email_address;
          sender.namedCells.get("email_notifications_service").callZome({
            zome_name: "email_notifications_service",
            fn_name: "request_send_email",
            payload: {
              email_address,
              email,
            },
          });
        }),
      ]);
    },
    true,
    { timeout: 520000 },
  );
});
