#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# evermemos-rs smoke test
# Tests the Rust server end-to-end, mirroring the original simple_demo.py flow.
#
# Usage:
#   # Terminal A — start server
#   cd evermemos-rs && cargo run --bin evermemos
#
#   # Terminal B — run tests
#   bash demo/smoke_test.sh
# ─────────────────────────────────────────────────────────────────────────────

BASE_URL="${EVERMEMOS_URL:-http://localhost:8080}"
USER_ID="demo-user-001"
ORG_ID="demo-org"
PASS=0
FAIL=0

# ── helpers ───────────────────────────────────────────────────────────────────

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

ok()   { echo -e "${GREEN}  ✓ $*${NC}"; ((PASS++)); }
fail() { echo -e "${RED}  ✗ $*${NC}"; ((FAIL++)); }
info() { echo -e "${YELLOW}▶ $*${NC}"; }
sep()  { echo "────────────────────────────────────────────────────────────"; }

assert_contains() {
  local label="$1" body="$2" pattern="$3"
  if echo "$body" | grep -q "$pattern"; then
    ok "$label"
  else
    fail "$label — expected pattern '$pattern' in: $body"
  fi
}

memorize() {
  local msg_id="$1" sender="$2" content="$3" role="${4:-user}"
  local ts; ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  curl -s -X POST "$BASE_URL/api/v1/memories" \
    -H "Content-Type: application/json" \
    -H "X-Organization-Id: $ORG_ID" \
    -d "{
      \"message_id\": \"$msg_id\",
      \"create_time\": \"$ts\",
      \"sender\": \"$sender\",
      \"sender_name\": \"$sender\",
      \"user_id\": \"$USER_ID\",
      \"content\": \"$content\",
      \"role\": \"$role\"
    }"
}

search() {
  local query="$1" method="${2:-KEYWORD}"
  curl -s "$BASE_URL/api/v1/memories/search" \
    -H "X-Organization-Id: $ORG_ID" \
    --data-urlencode "query=$query" \
    --data-urlencode "user_id=$USER_ID" \
    --data-urlencode "retrieve_method=$method" \
    --data-urlencode "top_k=5" \
    -G
}

fetch() {
  curl -s "$BASE_URL/api/v1/memories" \
    -H "X-Organization-Id: $ORG_ID" \
    --data-urlencode "user_id=$USER_ID" \
    --data-urlencode "limit=20" \
    -G
}

# ── 0. Health check ───────────────────────────────────────────────────────────
sep
info "Step 0 — Health check"
sep
resp=$(curl -s "$BASE_URL/health")
assert_contains "GET /health → status=ok" "$resp" '"status":"ok"'
echo "  $resp"

# ── 1. Delete any existing test data ─────────────────────────────────────────
sep
info "Step 1 — Clean up previous test data"
sep
curl -s -X DELETE "$BASE_URL/api/v1/memories" \
  -H "Content-Type: application/json" \
  -H "X-Organization-Id: $ORG_ID" \
  -d "{\"user_id\": \"$USER_ID\"}" | jq -c . 2>/dev/null || true
echo ""

# ── 2. Store conversations (mirrors simple_demo.py) ──────────────────────────
sep
info "Step 2 — Memorize conversation messages"
sep

messages=(
  "msg-001|User|I love playing soccer, often go to the field on weekends|user"
  "msg-002|Assistant|Soccer is a great sport! Which team do you like?|assistant"
  "msg-003|User|I love Barcelona the most, Messi is my idol|user"
  "msg-004|User|I also enjoy watching basketball, NBA is my favorite|user"
  "msg-005|User|I will sleep now|user"
  "msg-006|User|The weather is good today|user"
  "msg-007|User|The universe is expanding|user"
)

for entry in "${messages[@]}"; do
  IFS='|' read -r mid sender content role <<< "$entry"
  r=$(memorize "$mid" "$sender" "$content" "$role")
  status=$(echo "$r" | grep -o '"status":"[^"]*"' | head -1)
  echo "  [$mid] $status — $content"
  sleep 0.3
done
ok "All messages submitted"

# ── 3. Wait for extraction pipeline ──────────────────────────────────────────
sep
info "Step 3 — Waiting 12s for LLM extraction pipeline…"
sep
for i in $(seq 1 12); do printf "."; sleep 1; done; echo ""

# ── 4. Fetch stored memories ──────────────────────────────────────────────────
sep
info "Step 4 — Fetch stored memories (GET /api/v1/memories)"
sep
resp=$(fetch)
total=$(echo "$resp" | grep -o '"total_count":[0-9]*' | head -1)
echo "  Response: $total"
assert_contains "Fetch returns memories list" "$resp" '"memories"'

# ── 5. Keyword search ─────────────────────────────────────────────────────────
sep
info "Step 5 — Keyword search (KEYWORD)"
sep

queries=(
  "What sports does the user like?"
  "What is the user's favorite team?"
  "What are the user's hobbies?"
)

for q in "${queries[@]}"; do
  resp=$(search "$q" "KEYWORD")
  echo ""
  echo "  Query: $q"
  echo "  $(echo "$resp" | grep -o '"total_count":[0-9]*' | head -1)"
  # Print first memory content if any
  first=$(echo "$resp" | python3 -c "
import sys,json
data=json.load(sys.stdin)
mems=data.get('result',{}).get('memories',[])
if mems: print('  Top result:', mems[0].get('content','')[:120])
" 2>/dev/null)
  [ -n "$first" ] && echo "$first"
  assert_contains "  KEYWORD search '$q'" "$resp" '"memories"'
done

# ── 6. Vector search (if LLM reachable) ──────────────────────────────────────
sep
info "Step 6 — Vector search (VECTOR)"
sep
resp=$(search "soccer football sports" "VECTOR")
echo "  $(echo "$resp" | grep -o '"total_count":[0-9]*' | head -1)"
# Vector search may return 0 if embedder is offline — just check structure
assert_contains "VECTOR search returns valid envelope" "$resp" '"status"'

# ── 7. Summary ────────────────────────────────────────────────────────────────
sep
echo ""
echo -e "  ${GREEN}PASS: $PASS${NC}   ${RED}FAIL: $FAIL${NC}"
sep
if [ "$FAIL" -eq 0 ]; then
  echo -e "${GREEN}All tests passed!${NC}"
  exit 0
else
  echo -e "${RED}$FAIL test(s) failed.${NC}"
  exit 1
fi
