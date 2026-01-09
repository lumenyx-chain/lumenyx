# Community Bootnodes

This is the only file the community can edit. Submit a Pull Request to add your bootnode.

---

## How to add your bootnode:

1. Fork this repository
2. Add your bootnode to the table below
3. Submit a Pull Request
4. Wait for approval

---

## ⚠️ Warning

- All submissions are reviewed before approval
- Fake or malicious bootnodes will be rejected
- A fake bootnode cannot steal funds or hack your node - it simply won't connect
- Test your node is working before submitting

---

## How to get your Peer ID:
```bash
curl -s -H 'Content-Type: application/json' -d '{"id":1,"jsonrpc":"2.0","method":"system_localPeerId"}' http://127.0.0.1:9944
```

---

## Active Bootnodes:

| Name | IP | Port | Peer ID | Location |
|------|-----|------|---------|----------|
| *Add yours below* | | | | |

---

## Connection string format:
```
/ip4/IP_ADDRESS/tcp/30333/p2p/PEER_ID
```

---

The more bootnodes, the stronger the network!
