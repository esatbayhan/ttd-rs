#!/usr/bin/env bash
set -euo pipefail
# ---------------------------------------------------------------------------
# generate-demo-data.sh – create a fresh demo dataset for ttd screenshots
#
# Generates demo-data/todo.txt.d/ with tasks carrying relative dates so
# smart lists (Today, Tomorrow, Next Week, etc.) always evaluate correctly
# regardless of when the script is run.
# ---------------------------------------------------------------------------

if [[ $# -ne 0 ]]; then
  printf 'Usage: %s\n' "$0" >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ROOT="$PROJECT_DIR/demo-data/todo.txt.d"
rm -rf "$ROOT"
mkdir -p "$ROOT/done.txt.d" "$ROOT/lists.d"

TODAY=$(date +%Y-%m-%d)
YESTERDAY=$(date -d "-1 day"   +%Y-%m-%d)
TOMORROW=$(date -d "+1 day"    +%Y-%m-%d)
D_MINUS_30=$(date -d "-30 days" +%Y-%m-%d)
D_MINUS_14=$(date -d "-14 days" +%Y-%m-%d)
D_MINUS_7=$(date  -d "-7 days"  +%Y-%m-%d)
D_MINUS_3=$(date  -d "-3 days"  +%Y-%m-%d)
D_PLUS_3=$(date   -d "+3 days"  +%Y-%m-%d)
D_PLUS_7=$(date   -d "+7 days"  +%Y-%m-%d)
D_PLUS_14=$(date  -d "+14 days" +%Y-%m-%d)

n=0
while IFS= read -r line; do
  [ -z "$line" ] && continue
  n=$((n + 1))
  echo "$line" > "$ROOT/$(printf "demo-%04d.txt" "$n")"
done <<TASKS
(A) $TODAY Pay electricity bill +Home @desk due:$TODAY
(B) $TODAY Draft project proposal +Work @office due:$TOMORROW
(C) $TODAY Review Q3 budget +Work @office due:$D_PLUS_3 updated:$D_MINUS_30
(A) $TODAY Fix login bug +ttd @computer due:$TODAY updated:$TODAY
$TODAY Buy groceries +Personal @phone due:$YESTERDAY
$TODAY Call dentist +Personal @phone sched:$TOMORROW
$TODAY Plan team offsite +Work @office sched:$D_PLUS_7
$TODAY Read 'Atomic Habits' +Personal @home
$TODAY Organize desk drawer +Home @home updated:$TODAY
$TODAY Submit expense reports +Work @office due:$D_MINUS_3
$TODAY Renew domain +ttd @computer due:$D_PLUS_14
(A) $TODAY Prepare presentation +Work @office due:$D_PLUS_7
$TODAY Water plants +Home @home sched:$TODAY
$TODAY Update dependencies +ttd @computer
$TODAY Schedule 1:1 with manager +Work @office due:$D_PLUS_3
$TODAY Return library books +Personal @phone due:$D_MINUS_7 updated:$D_MINUS_14
$TODAY Write integration tests +ttd @computer sched:$D_PLUS_3
$TODAY Clean garage +Home @home due:$D_PLUS_14
$TODAY Order toner cartridge +Work @office due:$D_MINUS_14 updated:$D_MINUS_30
$TODAY Book flight for conference +Work @computer due:$D_PLUS_14
$TODAY Meditate 10 min +Personal @home
$TODAY Archive old emails +Work @office due:$D_MINUS_30
$TODAY Watch RustConf talk +Personal @computer
$TODAY Reply to vendor inquiry +Work @office due:$TOMORROW
$TODAY Backfill unit tests +ttd @computer due:$D_PLUS_7
TASKS

nd=0
while IFS= read -r line; do
  [ -z "$line" ] && continue
  nd=$((nd + 1))
  echo "$line" > "$ROOT/done.txt.d/$(printf "done-%04d.txt" "$nd")"
done <<DONE
x $YESTERDAY $D_MINUS_7 Submit timesheet +Work @office pri:A
x $D_MINUS_3 $D_MINUS_30 Learn vim basics +Personal @computer pri:B
x $D_MINUS_7 $D_MINUS_14 Migrate CI pipeline +ttd @computer pri:A
x $YESTERDAY $D_MINUS_3 Pick up dry cleaning +Errands @town pri:C
DONE

cat > "$ROOT/lists.d/0 Needs Review.list" <<'SMART'
---
name: Needs Review
icon: 🔍
description: Tasks not reviewed in 7 days – press u to review
---
no updated

OR

updated <= today - 7

sort by updated asc
SMART

cat > "$ROOT/lists.d/1 Today.list" <<'SMART'
---
name: Today
icon: 📅
description: Tasks due or scheduled for today, sorted by priority
---
due = today

OR

scheduled = today

sort by priority asc
SMART

cat > "$ROOT/lists.d/2 Tomorrow.list" <<'SMART'
---
name: Tomorrow
icon: 🌅
description: Tasks due tomorrow
---
due = today + 1

AND

not done

sort by due asc
SMART

cat > "$ROOT/lists.d/3 Next Week.list" <<'SMART'
---
name: Next Week
icon: 🗓️
description: Tasks due within the next 7 days
---
due <= today + 7

AND

not done

sort by due asc
SMART

cat > "$ROOT/lists.d/4 Overdue.list" <<'SMART'
---
name: Overdue
icon: ⚠️
description: Past-due tasks that need attention
---
due < today

AND

not done

sort by due asc
SMART

cat > "$ROOT/lists.d/5 Inbox.list" <<'SMART'
---
name: Inbox
icon: 📥
description: Tasks with no dates – triage later
---
no due

AND

no scheduled

AND

not done
SMART

cat > "$ROOT/lists.d/6 High Priority.list" <<'SMART'
---
name: High Priority
icon: 🔥
description: Priority A or B tasks
---
priority above C

sort by priority asc
sort by due asc
SMART

cat > "$ROOT/lists.d/7 Work.list" <<'SMART'
---
name: Work
icon: 🏢
description: Tasks tagged with +Work
---
project includes Work

AND

not done

sort by due asc
SMART

cat > "$ROOT/lists.d/8 By Project.list" <<'SMART'
---
name: By Project
icon: 📂
description: All open tasks grouped by project
---
has project

AND

not done

group by project asc
sort by priority asc
SMART

cat > "$ROOT/lists.d/9 This Month.list" <<'SMART'
---
name: This Month
icon: 📆
description: Tasks due or scheduled this month
---
due >= today - 30

AND

not done

sort by due asc
SMART

cat > "$ROOT/lists.d/A Done.list" <<'SMART'
---
name: Done
icon: ✅
description: Completed tasks
---
done

sort by description asc
SMART

echo "Generated demo dataset at $ROOT"
echo "  open tasks:   $(find "$ROOT" -maxdepth 1 -name '*.txt' | wc -l)"
echo "  done tasks:   $(find "$ROOT/done.txt.d" -name '*.txt' | wc -l)"
echo "  smart lists:  $(find "$ROOT/lists.d" -name '*.list' | wc -l)"
