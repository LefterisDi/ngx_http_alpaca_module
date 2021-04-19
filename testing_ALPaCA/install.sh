#!/bin/bash

VENVNAME=venv

virtualenv $VENVNAME
source ./$VENVNAME/bin/activate
pip install -r requirements.txt