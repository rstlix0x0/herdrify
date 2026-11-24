#!/bin/sh
# Probe: the spec §2 doc-discipline grep pattern, run with /usr/bin/grep, against words it must
# pass (left alone) and forms it must catch.
P='adrs?|specs?|plans? [0-9]+|milestones?|claudestacks|phases? [0-9]+|tasks? [0-9]+'
echo "\$ /usr/bin/grep --version"
/usr/bin/grep --version | head -1
echo "\$ printf '%s\\n' <case> | /usr/bin/grep -iwE '$P'"
for t in 'cargo run --example inspect' 'specific' 'a readr value' 'respect the order' \
         'see the spec' 'see the specs' 'per ADR 0002' 'per the ADRs' 'Plan 3 says' \
         'see plan 03' 'Task 12' 'Phase 10' 'the milestone' 'claudestacks-sdlc'; do
  if printf '%s\n' "$t" | /usr/bin/grep -iwE "$P" > /dev/null; then r=match; else r=none; fi
  printf '%-30s -> %s\n' "$t" "$r"
done
