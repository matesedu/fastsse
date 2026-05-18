const [crate, version] = process.argv.slice(2);

if (!crate || !version) {
  console.error("usage: node scripts/crate-version-published.mjs <crate> <version>");
  process.exit(2);
}

const response = await fetch(`https://crates.io/api/v1/crates/${crate}/${version}`, {
  headers: {
    "User-Agent": "matesedu/fastsse release workflow",
  },
});

if (response.status === 200) {
  process.exit(0);
}

if (response.status === 404) {
  process.exit(1);
}

console.error(`crates.io returned ${response.status} for ${crate}@${version}`);
process.exit(2);
