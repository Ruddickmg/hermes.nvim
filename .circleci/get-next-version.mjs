import semanticRelease from 'semantic-release';

try {
  console.log("running semantic release");
  const result = await semanticRelease({ dryRun: true });
  console.log("result", result);
  const version = result?.nextRelease?.version;
  console.log("retreived version!", version);

  if (version) {
    console.log("returning", version);
    process.stdout.write(version);
  } else {
    console.log("there was no version!");
    process.stderr.write('No next release version determined by semantic-release.\n');
  }
} catch (err) {
  console.log("an error happened", err);
  process.stderr.write(`semantic-release error: ${err.message}\n`);
  process.exit(1);
}
