# Mail-To-Telegram (MTL)
A small and efficient SMTP server for OpenMediaVault that will recieve notification emails and forwards them to a Telegram bot written in Rust. The docker container just uses around a few megabytes of RAM and uses async I/O and should be able to handle pretty much any notification load you throw at it. 

## Setup
The setup is pretty straightforward. Right now the only deployment method is docker. Alternatively you can build the server from source, but then you're on your own.

### Create a telegram bot
First you need a telegram bot. You create one by using the Telegram [BotFather](https://t.me/BotFather). You're gonna need the API Token the BotFather will give you.
### Find out the telegram chat id
This can be done with yet another bot called [ID Bot](https://t.me/username_to_id_bot). Just message them /start and it will tell you your personal chat id. If you want the chat id of a group chat, then just forward a message of that group chat to the ID Bot.
### Set the chat id as your email
MTL extracts the chat id from the recipient email. This means you need to set the email in the OpenMediaVault panel to YOUR_CHAT_ID@telegram-bot.com (e.g. 1234556789@telegram-bot.com).
### Deploy MTL
To deploy MTL you can use docker-compose or [yacht](yacht.sh). 
#### Docker-Compose
Create a `docker-compose.yml` file:
```yml
version: "3.7"

services:
    mtl:
      image: swip3798/mail-to-telegram:latest
      environment:
        TELEGRAM_BOT_TOKEN: "00000000:dhwaiuhdiuwahiudhwaiu" # Set this to your own API Token
        # ASYNC_STD_THREAD_COUNT: "4" # Add if you want to change the thread count used by the mtl server
        # STANDARD_CHAT_ID: "123456789" # Add if you want a fallback chat_id if the id can't be extracted from the recipient email
      ports:
        - 17333:17333 # Set to the wanted port
      restart: unless-stopped
```

Then deploy the server with 
```
docker-compose up -d
```
#### Yacht
Create a new template with this link:   
`https://raw.githubusercontent.com/swip3798/mail-to-telegram-app/master/yacht-template.json`
The just follow the normal deployment steps.

### Point OMV to your MTL instance
Now just go in your OMV panel to your notification settings, enable SMTP and set the IP and port to your MTL instance, if it's running on the same server then you can use `localhost` as your ip.

## Credits
The logo used for the yacht template is from [Freepik from Flaticon](https://www.flaticon.com/free-icons/mail).
