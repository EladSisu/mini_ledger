## This is a toy ledger CLI that process transactions list via csv and returns the account final balance state.

# Required csv input format and output format please view examples in tests dir.

# Assumptions :
1. All incoming transactions amount are positive. 
2. Transaction on a locked account are simply ignored.
3. The ledger would record all new clients transactions type, but set the available amount to 0 on a non deposit transaction.
4. Transaction History would be overridden by new transaction with the same transaction id.
5. Clients can dispute only withdrawal and deposit transactions types.
6. System would simply ignore bad transactions requests. i.e dispute tx with wrong client id.
7. There's no need to view old transaction history. (In this implementation only keep track of the latest action pre transaction)

# Future improvements/performance enhancements :
1. Add alert/warning output on bad transactions requests with proper error msg.
2. Since client's account is independent from other clients, for a very large csv that contains multiple clients. We can run a initial "filter process" to grab a transaction list of a particular client/s then, distribute the processing work across multiple threads such that each has all the clients' transactions history (keeping the original order).
3. same idea as #2 , but instead of using single process , we can replace the in memory tx list (hash map ) with a database. Then scale horizontally by distribute the work across multiple processes / machines.
