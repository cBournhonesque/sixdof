# Issues


## Networking

### Prespawning entities is causing rollbacks

- One issue caused by the Confirmed component not being spawned at the right time, see https://github.com/cBournhonesque/lightyear/issues/957. 
  - FIXED

- Another issue where the projectile is prespawned on the client at tick 1803, but somehow we receive a confirmed
update where the `confirmed.tick` is 1802? The server projectile is also spawned at tick 1803 so I don't understand how this
can happen.
  - FIXED

### Changing weapons is causing rollbacks

- Probably because the weapon change is not predicted correctly

### Replicating bullets from remote clients

- Do we want to replicate DespawnAfter? or the interpolated entity can add it themselves based on the weapon type?
- Current plan:
  - we don't even replicate the bullet to Predicted players; server/client spawn it independently and hopefully it should match
    (We can add PreSpawned to check that it doesn't cause any rollbacks)
  - for remote players, we replicate the `WeaponFiredEvent` component but not the other components (Position, LinearVelocity, etc.)
    That's because we only want to replicate the initial information. Then the remote client will spawn the bullet when their interpolation timeline reaches the correct tick
  - This means client-fired bullets live on the predicted timline but enemy bullets leave on the interpolated timeline, that could be weird?

## Performance

- Firing bullets continuously causes a 2X FPS drop
    