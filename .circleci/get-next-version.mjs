import semanticRelease from 'semantic-release';

try {
  const result = await semanticRelease({ dryRun: true });
  const version = result?.nextRelease?.version;

  if (version) {
    process.stdout.write(version);
  } else {
    // write empty string to stdout so that the last line is an empty string
    process.stdout.write("");
    process.stderr.write('No next release version determined by semantic-release.\n');
  }
} catch (err) {
  process.stderr.write(`semantic-release error: ${err.message}\n`);
  process.exit(1);
}
