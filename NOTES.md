# Issues


## Networking

### Prespawning entities is causing rollbacks

- One issue caused by the Confirmed component not being spawned at the right time, see https://github.com/cBournhonesque/lightyear/issues/957. 
  - FIXED

- Another issue where the projectile is prespawned on the client at tick 1803, but somehow we receive a confirmed
update where the `confirmed.tick` is 1802? The server projectile is also spawned at tick 1803 so I don't understand how this
can happen.
  - It looks like the Update we receive is for an EARLIER tick than the Spawn message!? That's crazy I thought I was guaranteeing
    that the Update would be from a tick later than the spawn. Also it's straight up not possible since the server also spawned the
    entity on tick 1803.
  - I receive packet for Group1 with Action-tick 1842. (entities = player + projectile)
  - I receive packet for Group1 with Update-tick 1840, last-action-tick = 1749. (entity = player)
    - updates the confirmed_tick for all entities in the group to 1840.
  - It seems like we have logic in `read_messages` to ignore Update messages older than the latest_tick,
    but we were missing logic in `apply_world` to ignore Update messages that are older than the latest_tick (which is updated
    just above when we apply an Action message in the same tick)
  - FIXED

### Changing weapons is causing rollbacks

- Probably because the weapon change is not predicted correctly

## Performance

- Firing bullets continuously causes a 2X FPS drop
    