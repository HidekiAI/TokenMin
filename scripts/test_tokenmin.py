import sqlite3
import time
import os
import hashlib
import hmac

db_path = "/dev/shm/chat_and_plan/message_queue.sqlite3"
hmac_secret = os.environ.get("TOKENMIN_HMAC_SECRET", "default_secret").encode('utf-8')

# Wait a second to ensure DB is initialized
time.sleep(1)
if not os.path.exists(db_path):
    print(f"Database {db_path} does not exist yet. Did the daemon start correctly?")
    exit(1)

conn = sqlite3.connect(db_path)
cur = conn.cursor()

raw_text = """Hey there! I was just wondering if you could help me out. This is just a lot of conversational filler to see if the model actually summarizes it down.
Anyway, here is the core logic:
```rust
fn main() {
    println!("Hello, world!");
}
```
Thanks so much for taking a look at this. It really helps a lot."""

def compute_checksum(text):
    return hmac.new(hmac_secret, text.encode('utf-8'), hashlib.sha256).hexdigest()

now_ms = int(time.time() * 1000)

# Test 1: Full Compaction
checksum1 = compute_checksum(raw_text)
cur.execute('''
    INSERT INTO messages (session_id, role, raw_content, status, model, hmac_signature, created_at)
    VALUES (?, ?, ?, ?, ?, ?, ?)
''', ('test_session', 'user', raw_text, 'pending', 'gpt-4', checksum1, now_ms))

msg_id = cur.lastrowid
conn.commit()
print(f"[Test 1] Inserted message for compaction with ID: {msg_id}")

timeout = 20
for i in range(timeout):
    cur.execute('SELECT status, processed_content FROM messages WHERE id = ?', (msg_id,))
    row = cur.fetchone()
    if row[0] == 'completed':
        print(f"\n✅ Message processed successfully!")
        print(f"Status: {row[0]}")
        print(f"Original Length: {len(raw_text)}")
        print(f"Processed Length: {len(row[1]) if row[1] else 0}")
        print(f"Processed Content:\n{'-'*30}\n{row[1]}\n{'-'*30}")
        break
    elif row[0] in ('failed', 'skipped'):
        print(f"\n❌ Message processing finished with unexpected status: {row[0]}")
        print(f"Processed Content: {row[1]}")
        break
    print(f"Status is '{row[0]}', waiting...")
    time.sleep(1)
else:
    print(f"Timeout waiting for message to process. Last status: {row[0]}")

# Test 2: Compaction Bypass
bypass_text = 'Bypass this! No changes needed.'
checksum2 = compute_checksum(bypass_text)
cur.execute('''
    INSERT INTO messages (session_id, role, raw_content, status, model, hmac_signature, created_at)
    VALUES (?, ?, ?, ?, ?, ?, ?)
''', ('test_session2', 'user', bypass_text, 'pending', 'copilot-chat', checksum2, now_ms + 1))
bypass_id = cur.lastrowid
conn.commit()
print(f"\n[Test 2] Inserted bypass message with ID: {bypass_id}")

for i in range(timeout):
    cur.execute('SELECT status, processed_content FROM messages WHERE id = ?', (bypass_id,))
    row = cur.fetchone()
    if row[0] == 'skipped':
        print(f"\n✅ Bypass message processed successfully!")
        print(f"Status: {row[0]}")
        print(f"Processed Content: {row[1]}")
        break
    time.sleep(1)
else:
    print(f"Timeout waiting for bypass message to process. Last status: {row[0]}")

conn.close()
