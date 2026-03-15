import asyncio
import json
import os
import sys
from datetime import datetime

# Add the current directory to sys.path so we can import the client
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from evermemos_agent_lib.client import EverMemOSClient

async def wait_for_profile(client, user_id, timeout=300, interval=5, expected_keyword=None):
    """Poll for profile availability with a timeout."""
    start_time = datetime.now()
    print(f"  Polling for profile (timeout {timeout}s)...")
    while (datetime.now() - start_time).total_seconds() < timeout:
        try:
            profile = await client.get_global_profile(user_id=user_id)
            print(f"    [Poll Debug] Fetched profile: {profile}")
            # Check if profile_data is not empty
            if profile and profile.get("profile_data"):
                p_data = profile.get("profile_data", {})
                if p_data and len(p_data.keys()) > 0:
                    if expected_keyword:
                        # flatten and check
                        up_str = json.dumps(p_data, ensure_ascii=False).lower()
                        if expected_keyword.lower() in up_str:
                            return profile
                    else:
                        return profile
        except Exception as e:
            print(f"    [Poll Debug] Exception: {e}")
            # 404 or other errors are expected before extraction is done
            pass
        
        await asyncio.sleep(interval)
    return None

async def trigger_boundary(client, user_name):
    """Send a topic change message to force the boundary detector to trigger."""
    print("  Sending topic change signal to trigger extraction...")
    await client.add_conversation(
        user_message="好了，关于我的基本情况就先聊到这吧。我们换个话题。",
        assistant_message="好的，没问题！你想聊点别的什么吗？",
        user_name=user_name
    )

async def test_metabolism():
    # Configuration
    base_url = "http://localhost:8080"
    # Use a dynamic user_id to avoid data pollution from previous runs
    user_id = f"test_user_metabolism_{int(datetime.now().timestamp())}"
    # Use user_id as the name so LLM knows exactly who this is about
    user_name = user_id
    
    # Increase timeout to handle slow LLM boundary detection
    client = EverMemOSClient(base_url=base_url, user_id=user_id, timeout=300.0)
    
    print(f"--- Starting Metabolism Test for user: {user_name} ({user_id}) ---")

    # Accumulate history manually for the test
    full_history = []

    # Step 1: Initial context - Personality and Coffee preference
    print("\n[Step 1] Sending initial preference: Coffee")
    msg1_user = f"你好，我是{user_name}。我是一个性格比较安静的人，平时最喜欢喝咖啡，每天都要喝两杯拿铁。"
    msg1_asst = f"你好{user_name}！很高兴认识你。安静的性格配上一杯拿铁确实很惬意，看来你是个深度咖啡爱好者呢。"
    
    full_history.extend([
        {"role": "user", "content": msg1_user, "sender": user_name},
        {"role": "assistant", "content": msg1_asst, "sender": "Assistant"}
    ])

    # We bypass add_conversation and directly call memorize to have exact control over history
    print("  Sending topic change signal to trigger extraction with full history...")
    trigger_user = "好了，关于我的基本情况就先聊到这吧。我们换个话题。"
    
    await client.memorize(
        content=trigger_user,
        sender=user_name,
        role="user",
        history=full_history
    )
    
    full_history.append({"role": "user", "content": trigger_user, "sender": user_name})
    
    trigger_asst = "好的，没问题！你想聊点别的什么吗？"
    await client.memorize(
        content=trigger_asst,
        sender="Assistant",
        role="assistant",
        history=full_history
    )

    print("Waiting for initial profile extraction...")
    profile = await wait_for_profile(client, user_id)
    
    if not profile:
        print("❌ FAILURE: Initial profile was never extracted (Check server logs for LLM errors).")
        await client.aclose()
        return

    print("\n[Initial Profile Result]:")
    print(json.dumps(profile, indent=2, ensure_ascii=False))
    
    # Step 2: Evolution context - Changing preference to Green Tea due to health
    print("\n[Step 2] Sending evolved preference: Green Tea (Conflict with Coffee)")
    
    # Reset history for the new topic segment
    full_history = []
    
    msg2_user = "最近我发现喝咖啡老是失眠，医生建议我戒掉咖啡。我现在改喝绿茶了，发现绿茶清淡的味道也很适合我，现在已经完全不碰咖啡了。"
    msg2_asst = "身体健康最重要。从咖啡转向绿茶是个不错的变化，绿茶确实清淡健康，对改善睡眠很有帮助。希望你的睡眠能因此好起来！"
    
    full_history.extend([
        {"role": "user", "content": msg2_user, "sender": user_name},
        {"role": "assistant", "content": msg2_asst, "sender": "Assistant"}
    ])

    print("  Sending topic change signal to trigger extraction with full history...")
    trigger_user2 = "关于喝茶的事就说到这吧。"
    await client.memorize(
        content=trigger_user2,
        sender=user_name,
        role="user",
        history=full_history
    )
    
    full_history.append({"role": "user", "content": trigger_user2, "sender": user_name})
    
    trigger_asst2 = "好的，我们聊点别的。"
    await client.memorize(
        content=trigger_asst2,
        sender="Assistant",
        role="assistant",
        history=full_history
    )

    print("Waiting for evolved profile extraction...")
    # Polling will detect the update
    updated_profile = await wait_for_profile(client, user_id, expected_keyword="茶")
    
    if not updated_profile:
        print("❌ FAILURE: Updated profile was not available.")
        await client.aclose()
        return

    print("\n[Updated Profile Result]:")
    print(json.dumps(updated_profile, indent=2, ensure_ascii=False))
    
    up_data = updated_profile.get("profile_data", {})
    # Flatten structure for easy checking
    up_str = json.dumps(up_data, ensure_ascii=False).lower()
    
    print(f"Updated Profile Data: {up_str}")
    
    # Validation logic
    has_tea = "绿茶" in up_str or "tea" in up_str
    # The metabolism logic should either remove coffee or mark it as discontinued
    has_coffee = "咖啡" in up_str or "coffee" in up_str
    
    if has_tea:
        print("\n✅ SUCCESS: Profile successfully evolved to include Green Tea!")
    else:
        print("\n❌ FAILURE: Profile did not reflect the evolution to Green Tea.")

    if has_coffee:
        # If coffee is still present, check if it's marked as "past" or "no longer"
        if "不" in up_str or "戒" in up_str or "old" in up_str or "used to" in up_str or "替代" in up_str:
            print("✅ SUCCESS: Old preference (Coffee) was correctly acknowledged as discontinued/past.")
        else:
            print("⚠️ WARNING: Coffee is still in profile without clear discontinued status.")
    else:
        print("✅ SUCCESS: Old preference (Coffee) was successfully metabolized (removed).")

    await client.aclose()

if __name__ == "__main__":
    asyncio.run(test_metabolism())
