import os

import requests

def test_rerank_backend():
    base_url = "https://api.cohere.com/v2/rerank"
    api_key = os.getenv("COHERE_API_KEY")

    if not api_key:
        print("Error testing rerank backend: COHERE_API_KEY is not set")
        return

    headers = {
        "Authorization": f"bearer {api_key}",
        "Content-Type": "application/json",
        "accept": "application/json"
    }

    payload = {
        "model": "rerank-v4.0-pro",
        "query": "What is the capital of the United States?",
        "top_n": 3,
        "documents": [
            "Carson City is the capital city of the American state of Nevada.",
            "The Commonwealth of the Northern Mariana Islands is a group of islands in the Pacific Ocean. Its capital is Saipan.",
            "Washington, D.C. (also known as simply Washington or D.C., and officially as the District of Columbia) is the capital of the United States. It is a federal district.",
            "Capitalization or capitalisation in English grammar is the use of a capital letter at the start of a word. English usage varies from capitalization in other languages.",
            "Capital punishment has existed in the United States since before the United States was a country. As of 2017, capital punishment is legal in 30 of the 50 states."
        ]
    }

    try:
        response = requests.post(base_url, headers=headers, json=payload)
        response.raise_for_status()
        print("Rerank backend response:")
        print(response.json())
    except requests.exceptions.RequestException as e:
        print(f"Error testing rerank backend: {e}")

if __name__ == "__main__":
    test_rerank_backend()