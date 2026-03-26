import semanticRelease from 'semantic-release';

try {
  const result = await semanticRelease({ dryRun: true });
  const version = result?.nextRelease?.version;

  if (version) {
    process.stdout.write(version);
  } else {
    process.stderr.write('No next release version determined by semantic-release.\n');
  }
} catch (err) {
  process.stderr.write(`semantic-release error: ${err.message}\n`);
  process.exit(1);
}
