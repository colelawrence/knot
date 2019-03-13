module Main exposing (Model, Msg(..), init, main, subscriptions, update, view)

import Browser
import Html exposing (Html, pre, text, a)
import Html.Attributes exposing (href)
import Http
import Json.Decode exposing (Decoder, field, string)


authPublicUrl = "https://901.rkt.space"


-- MAIN


main =
    Browser.element
        { init = init
        , update = update
        , subscriptions = subscriptions
        , view = view
        }



-- MODEL

type StringResult
    = Failure
    | Loading
    | Success String

type alias Model =
    { loginAccess : StringResult
    , loginUrl : StringResult
    }


init : () -> ( Model, Cmd Msg )
init _ =
    ( Model Loading Loading
    , getLoginAccessToken
    )



-- UPDATE


type Msg
    = GotLoginToken (Result Http.Error String)
    | GotLoginUrl (Result Http.Error String)


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotLoginToken result ->
            case result of
                Ok token ->
                    ( { model | loginAccess = Success token }, getLoginUrl token )

                Err _ ->
                    ( { model | loginAccess = Failure }, Cmd.none )
        GotLoginUrl result ->
            case result of
                Ok url ->
                    ( { model | loginUrl = Success url }, Cmd.none )

                Err _ ->
                    ( { model | loginUrl = Failure }, Cmd.none )



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.none



-- VIEW


view : Model -> Html Msg
view model =
    case model.loginUrl of
        Failure ->
            text "I was unable to load your book."

        Loading ->
            text "Loading..."

        Success loginUrl ->
            a [ href loginUrl ] [ text "Login with Google" ]



-- HTTP

getLoginAccessToken : Cmd Msg
getLoginAccessToken =
    Http.post
        { body = Http.emptyBody
        , url = authPublicUrl ++ "/auth/v0/login/session"
        , expect = Http.expectJson GotLoginToken tokenDecoder
        }

getLoginUrl : String -> Cmd Msg
getLoginUrl accessToken =
    Http.request
        { method = "POST"
        , body = Http.emptyBody
        , headers = [ Http.header "Authorization" ("Bearer " ++ accessToken) ]
        , url = authPublicUrl ++ "/auth/v0/google/login_url"
        , expect = Http.expectJson GotLoginUrl urlDecoder
        , timeout = Nothing
        , tracker = Nothing
        }


tokenDecoder : Decoder String
tokenDecoder =
    field "access_token" string

urlDecoder : Decoder String
urlDecoder =
    field "url" string