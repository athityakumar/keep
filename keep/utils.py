"""Utility functions of the cli."""
import json
import os
import random
import string
import sys
import time
import click
import requests

# Directory for Keep files
dir_path = os.path.join(os.path.expanduser('~'), '.keep')


def first_time_use(ctx):
    click.secho("Initializing environment in ~/.keep directory", fg='green')
    for i in range(2):
        click.echo('.', nl=False)
        time.sleep(0.5)
    click.echo('.OK', nl=True)

    os.mkdir(dir_path)

    register()
    sys.exit(0)


def log(ctx, message):
    """Prints log when verbose set to True."""
    if ctx.verbose:
        ctx.log(message)


def register():
    # User may not choose to register and work locally.
    # Registration is required to push the commands to server
    if click.confirm('Proceed to register?', abort=True, default=True):
        # Verify for existing user
        click.echo("Your credentials will be saved in the ~/.keep directory.")
        email = click.prompt('Email', confirmation_prompt=True)
        json_res = {'email': email}
        click.echo('Verifying with existing users...')
        r = requests.post('https://keep-cli.herokuapp.com/check-user', json=json_res)
        if r.json()['exists']:
            click.secho('User already exists !', fg='red')
            email = click.prompt('Email', confirmation_prompt=True)
            json_res = {'email': email}
            r = requests.post('https://keep-cli.herokuapp.com/check-user', json=json_res)
        # Generate password for the user
        chars = string.ascii_letters + string.digits
        password = ''.join(random.choice(chars) for _ in range(255))
        credentials_file = os.path.join(dir_path, '.credentials')
        credentials = {
            'email': email,
            'password': password
        }
        click.secho("Generated password for " + email, fg='cyan')
        # Register over the server
        click.echo("Registering new user ...")
        json_res = {
            'email': email,
            'password': password
        }
        r = requests.post('https://keep-cli.herokuapp.com/register', json=json_res)
        if r.status_code == 200:
            click.secho("User successfully registered !", fg='green')
            # Save the credentials into a file
            with open(credentials_file, 'w+') as f:
                f.write(json.dumps(credentials))
            click.secho(password, fg='cyan')
            click.secho("Credentials file saved at ~/.keep/.credentials", fg='green')
    sys.exit(0)


def remove_command(ctx, cmd):
    json_path = os.path.join(dir_path, 'commands.json')
    commands = {}
    if os.path.exists(json_path):
        commands = json.loads(open(json_path, 'r').read())
    else:
        click.echo('No commands to remove. Run `keep new` to add one.')

    if cmd in commands:
        del commands[cmd]
        click.echo('Command successfully removed!')
        with open(json_path, 'w') as f:
            f.write(json.dumps(commands))
    else:
        click.echo('Command - {} - does not exist.'.format(cmd))


def save_command(cmd, desc):
    json_path = os.path.join(dir_path, 'commands.json')
    commands = {}
    if os.path.exists(json_path):
        commands = json.loads(open(json_path, 'r').read())
    commands[cmd] = desc
    with open(json_path, 'w') as f:
        f.write(json.dumps(commands))
