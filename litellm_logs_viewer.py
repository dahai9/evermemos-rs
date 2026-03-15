#!/usr/bin/env python3
import urllib.request
import urllib.parse
import json
import sys
import argparse
from datetime import datetime, timedelta, timezone

def get_logs(hours_ago=24, page_size=20, page=1):
    now = datetime.now(timezone.utc)
    start = now - timedelta(hours=hours_ago)
    
    # API requires dates in format %Y-%m-%d %H:%M:%S
    start_str = start.strftime('%Y-%m-%d %H:%M:%S')
    end_str = now.strftime('%Y-%m-%d %H:%M:%S')
    
    url = f"http://127.0.0.1:4000/spend/logs/ui?start_date={urllib.parse.quote(start_str)}&end_date={urllib.parse.quote(end_str)}&page={page}&page_size={page_size}"
    
    req = urllib.request.Request(url, headers={
        "Authorization": "Bearer sk-SJS-qFOpvosu4xoOHhJ-Ww",
        "Content-Type": "application/json"
    })
    
    try:
        with urllib.request.urlopen(req) as response:
            return json.loads(response.read().decode())
    except urllib.error.HTTPError as e:
        print(f"Error fetching logs: {e.read().decode()}")
        sys.exit(1)

def print_logs(data):
    if 'data' not in data:
        print("No data found or unexpected format.")
        return
        
    for item in data['data']:
        print("=" * 80)
        call_type = item.get('call_type', 'unknown')
        model = item.get('model', 'unknown')
        start = item.get('startTime', '')
        end = item.get('endTime', '')
        tokens = item.get('total_tokens', 0)
        
        print(f"Time: {start} -> {end}")
        print(f"Type: {call_type} | Model: {model} | Total Tokens: {tokens}")
        
        req_data = item.get('proxy_server_request', {})
        messages = item.get('messages', None)
        
        print("-" * 40)
        print("REQUEST:")
        if call_type == 'aembedding' or call_type == 'embedding':
            input_text = req_data.get('input', '')
            if isinstance(input_text, list):
                # Only show the first item if it's a batch, and truncate
                print(f"Input (Batch of {len(input_text)}): {str(input_text[0])[:500]}... (truncated)")
            else:
                if isinstance(input_text, str) and len(input_text) > 1000:
                    input_text = input_text[:1000] + "... [TRUNCATED]"
                print(f"Input: {input_text}")
        elif req_data and 'messages' in req_data:
            msgs = req_data['messages']
            for m in msgs:
                role = m.get('role', '')
                content = m.get('content', '')
                if isinstance(content, str) and len(content) > 1000:
                    content = content[:1000] + "... [TRUNCATED]"
                print(f"[{role.upper()}]: {content}")
        elif messages and isinstance(messages, list):
            for m in messages:
                if isinstance(m, dict):
                    role = m.get('role', '')
                    content = m.get('content', '')
                    if isinstance(content, str) and len(content) > 1000:
                        content = content[:1000] + "... [TRUNCATED]"
                    print(f"[{role.upper()}]: {content}")
        else:
            req_str = json.dumps(req_data, indent=2, ensure_ascii=False)
            print(req_str[:1000] + ("..." if len(req_str) > 1000 else ""))
            
        print("-" * 40)
        print("RESPONSE:")
        resp = item.get('response', {})
        if call_type == 'aembedding' or call_type == 'embedding':
            print("<Embedding Data Hidden>")
        else:
            choices = resp.get('choices', [])
            if choices:
                for c in choices:
                    msg = c.get('message', {})
                    role = msg.get('role', '')
                    content = msg.get('content', '')
                    if isinstance(content, str) and len(content) > 1000:
                        content = content[:1000] + "... [TRUNCATED]"
                    print(f"[{role.upper()}]: {content}")
            else:
                # Fallback for unexpected response structure
                resp_str = json.dumps(resp, indent=2, ensure_ascii=False)
                print(resp_str[:1000] + ("..." if len(resp_str) > 1000 else ""))
                
    print("=" * 80)
    print(f"Page {data.get('page')} of {data.get('total_pages')}. Total entries: {data.get('total')}")

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description="Fetch and view LiteLLM logs.")
    parser.add_argument("--hours", type=int, default=24, help="Fetch logs from last N hours")
    parser.add_argument("--size", type=int, default=5, help="Number of logs per page")
    parser.add_argument("--page", type=int, default=1, help="Page number")
    
    args = parser.parse_args()
    
    data = get_logs(hours_ago=args.hours, page_size=args.size, page=args.page)
    print_logs(data)
