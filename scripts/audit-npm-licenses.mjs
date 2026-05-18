import { execFileSync } from "node:child_process";

const allowedLicenses = new Set([
  "Apache-2.0",
  "BSD-3-Clause",
  "ISC",
  "MIT",
  "MPL-2.0",
  "Python-2.0",
]);

const raw = execFileSync("pnpm", ["licenses", "list", "--json", "--dev"], {
  encoding: "utf8",
});
const licenses = JSON.parse(raw);
const disallowed = Object.keys(licenses)
  .filter((license) => !allowedLicenses.has(license))
  .sort();

if (disallowed.length > 0) {
  for (const license of disallowed) {
    console.error(`disallowed npm license: ${license}`);
    for (const pkg of licenses[license]) {
      console.error(`  - ${pkg.name}@${pkg.versions.join(", ")}`);
    }
  }
  console.error("Update scripts/audit-npm-licenses.mjs only after maintainer review.");
  process.exit(1);
}

console.log(`npm license audit passed for ${Object.keys(licenses).length} license groups`);
