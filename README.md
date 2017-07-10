# [Parity](https://ethcore.io/parity.html) and Ouroboros

## Caveats

This implementation has been created for the purpose of measuring performance
of the Ouroboros algorithm on the Parity platform in terms of transactions per
second. Parts of the protocol that handle resilience, bad actors, etc have not
been implemented since the nodes are all known to be online and honest during
the performance test runs.

## Glossary

| Ouroboros term | Parity term |
|----------------|-------------|
| Slot           | Step        |
| Stakeholder/Leader | Validator |
