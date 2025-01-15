function OnLoad()
    print("Welcome to Quantsum!")
end

function OnEnterPlayingState(map_name, is_server)
    if is_server then
        local monster = {
            kind = "GruntLaser",
            scale = { x = 2, y = 2, z = 2 },
        }
        SpawnMonster(monster, {
            x = 1,
            y = 0,
            z = 1,
        })
        SpawnMonster(monster, {
            x = 1,
            y = 3,
            z = 1,
        })
        SpawnMonster(monster, {
            x = 1,
            y = 2,
            z = 1,
        })
    end
end

function OnPlayerFraggedPlayer(player, fraggedPlayer)
    if player.id == fraggedPlayer.id then
        -- score is stored as a u16 atm, so we need to check for overflow
        -- @todo: make this an i16 instead
        player.score = player.score - 1
        if player.score < 0 then
            player.score = 0
        end

        player.deaths = player.deaths + 1
        UpdatePlayer(player)
        --RespawnPlayer(player.id, 3.0)
    else
        player.score = player.score + 1
        player.frags = player.frags + 1
        fraggedPlayer.deaths = fraggedPlayer.deaths + 1
        UpdatePlayer(player)
        UpdatePlayer(fraggedPlayer)
        --RespawnPlayer(fraggedPlayer.id, 3.0)
    end
end

function OnPlayerFraggedMonster(player, fraggedMonster)
    player.score = player.score + 1
    player.frags = player.frags + 1
    UpdatePlayer(player)
    SpawnHealth(15, fraggedMonster.translation)
end

function OnMonsterFraggedPlayer(monster, fraggedPlayer)
    fraggedPlayer.deaths = fraggedPlayer.deaths + 1
    --UpdatePlayer(fraggedPlayer)
    --RespawnPlayer(fraggedPlayer.id, 3.0)
end

function OnMonsterFraggedMonster(monster, fraggedMonster)
    SpawnHealth(15, fraggedMonster.translation)
end
