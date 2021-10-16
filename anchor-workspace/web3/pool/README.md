## Список всех пулов

`getPools(provider)`

* `provider` -- провайдер. Пример настройки провайдера:

```js
import { useWallet } from '@solana/wallet-adapter-react';

const wallet = useWallet();

async function getProvider() {
  const opts = {
    preflightCommitment: "processed",
  };
  // меняем на девнет или другую сеть
  const network = "http://127.0.0.1:8899";
  const connection = new Connection(network, opts.preflightCommitment);

  // wallet достаем
  const provider = new Provider(
      connection, wallet, opts.preflightCommitment
  );
  return provider;
}
```

Возвращает список пулов вида:

```js
[
    Pool {
        publicKey: PublicKey { ... },
        poolAuthority: [PublicKey],
        administratorAuthority: [PublicKey],
        nonce: 255,
        genesis: <BN: 6160858d>,
        topupDuration: <BN: 3>,
        lockupDuration: <BN: 6>,
        stakeAcquiredAmount: <BN: 0>,
        stakeTargetAmount: <BN: 2710>,
        rewardAmount: <BN: 64>,
        depositedRewardAmount: <BN: 0>,
        stakeMint: [PublicKey],
        stakeVault: [PublicKey]
    }
]
```

У объектов-пулов есть методы, описанные ниже.

## Расчет APY для пула

Доступно два метода:

`pool.expectedAPY()`

`pool.APY()`

* `pool` -- это объект из запроса `getPools`.

`expectedAPY` рассчитывает APY при выполнении всех целевых показателей, то есть в случае
когда весь пул заполнен и начислены все награды.

`APY` рассчитывает APY на текущий момент, то есть учитываются текущие суммы наград и заполненности пула.

## Методы для получения информации о пуле

`pool.totalPoolDeposits()`

`pool.maxPoolSize()`

`pool.totalRewards()`

`pool.rewardsRemaining()`

`pool.startDate()` -- возвращает объекта типа `Date`, описывающий дату старта пула.

`pool.endDate()` -- возвращает объект типа `Date`, описывающий дату окончания пула.

`pool.timeToDeposit()` -- время в секундах до окончания времени приема депозитов.

`pool.timeUntilWithdrawal()` -- время в секундах до появления возможности у стейкеров забрать свои депозиты с наградами.


## Добавление стейка

`addStake(provider, amount, ticket, accounts)`

* `provider` -- то же, что и в `getPools`.

* `amount` -- количество токенов, которые нужно отправить в пул. Здесь и далее это значение инициализируется следующим образом:

```js
const anchor = require('@project-serum/anchor');
const amount = new anchor.BN(100);
```

* `ticket` -- Keypair для управления билетом стейкера. Генерируется следующим образом:

```js
const anchor = require('@project-serum/anchor');
const ticket = anchor.web3.Keypair.generate();
```

Скорее всего, этот момент будет переработан, а аргумент `ticket` исчезнет.

* `accounts`:

```js
{
    // известен из запроса getPools,
    pool: pool.publicKey,
    // известен из запроса getPools
    stakeVault: pool.stakeVault,
    // это адрес кошелька пользователя
    sourceAuthority: provider.wallet.publicKey,
    // кошелек, указанный пользователем
    sourceWallet: some.publicKey,
}
```

## Удаление стейка

`removeStake(provider, amount, accounts)`

* `provider` -- то же, что в `getPools`.
* `amount` -- то же, что и в `addStake`.
* `accounts`:

```js
{
    // известен из запроса getPools,
    pool: pool.publicKey,
    // известен из запроса getPools
    stakeVault: pool.stakeVault,
    // это адрес кошелька пользователя
    staker: provider.wallet.publicKey,
    // кошелек, указанный пользователем
    targetWallet: some.publicKey,
    // адрес билета
    ticket: ticket.publicKey,
    // это нужно узнать у разработчиков
    // возможно, будет переработано
    poolAuthority: some.publicKey,
}
```

## Сбор наград

`claimReward(provider, accounts)`

* `provider` -- то же, что и в `getPools`.
* `accounts`:

```js
{
    // известен из запроса getPools,
    pool: pool.publicKey,
    // известен из запроса getPools
    stakeVault: pool.stakeVault,
    // это адрес кошелька пользователя
    staker: provider.wallet.publicKey,
    // кошелек, указанный пользователем
    targetWallet: some.publicKey,
    // адрес билета
    ticket: ticket.publicKey,
    // это нужно узнать у разработчиков
    // возможно, будет переработано
    poolAuthority: some.publicKey,
}
```