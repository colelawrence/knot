port module Main exposing (Auth(..), FlashMessages, LoginMsg(..), LoginSession, Model, Msg(..), PersistentState, StringResult(..), UserMsg(..), UserSession, UserSessionJson, applyMessages, authPublicUrl, authRequest, defaultPersistentState, error, getLoginAccessToken, getLoginUrl, init, initUserSession, initializeWithUserToken, main, meDecoder, setUserToken, subscriptions, success, tokenDecoder, update, updateLoading, updateLoginSession, updateUserSession, updateWith, updateWithStorage, urlDecoder, userSessionInit, view, viewPage, viewUser)

import Browser
import Html exposing (Html, a, pre, text)
import Html.Attributes exposing (href)
import Html.Events exposing (onClick)
import Http
import Json.Decode as JD exposing (Decoder, field, string)
import Json.Encode as JE
import Task


authPublicUrl =
    "https://901.rkt.space"


appPublicUrl =
    "http://localhost:8000"



-- MAIN


main =
    Browser.element
        { init = init
        , update = updateWithStorage
        , subscriptions = subscriptions
        , view = view
        }


port setUserToken : Maybe String -> Cmd msg


port setLoginToken : Maybe String -> Cmd msg


{-| We want to `setUserToken` on every update. This function adds the setUserToken
command for every step of the update function.
-}
updateWithStorage : Msg -> Model -> ( Model, Cmd Msg )
updateWithStorage msg model =
    let
        ( newModel, cmds ) =
            update msg model
    in
    case newModel.auth of
        NotLoggedIn { loginToken } ->
            ( newModel
            , Cmd.batch [ setLoginToken (Just loginToken), cmds ]
            )

        LoggedIn { userToken } ->
            ( newModel
            , Cmd.batch [ setUserToken (Just userToken), cmds ]
            )

        _ ->
            ( newModel
            , Cmd.batch [ setUserToken Nothing, setLoginToken Nothing, cmds ]
            )



-- MODEL


type StringResult
    = Failure
    | Inactive
    | Success String


type alias PersistentState =
    { userToken : Maybe String
    , loginToken : Maybe String
    }


defaultPersistentState =
    PersistentState Nothing


type Auth
    = NotLoggedIn LoginSession
    | LoggedIn UserSession
    | Loading


type alias UserSession =
    { userToken : String
    , userName : String
    , userPhotoUrl : String
    }


type alias UserSessionJson =
    { userToken : String
    , userName : String
    , userPhotoUrl : String
    }


type alias LoginSessionJson =
    { loginToken : String
    , userId : Maybe String
    , displayName : Maybe String
    }


userSessionInit : UserSessionJson -> UserSession
userSessionInit session =
    UserSession session.userToken session.userName session.userPhotoUrl


type alias LoginSession =
    { loginToken : String
    , loginUrl : StringResult
    , userId : Maybe String
    }


loginSessionInit : String -> LoginSession
loginSessionInit token =
    LoginSession token Inactive Nothing


type alias FlashMessages =
    List String


type alias Model =
    { auth : Auth
    , messages : FlashMessages
    }


init : Maybe PersistentState -> ( Model, Cmd Msg )
init maybePersistentState =
    let
        defaultLoading =
            ( Model Loading []
            , getLoginAccessToken
            )
    in
    case maybePersistentState of
        Just persistentState ->
            case persistentState.userToken of
                Just userToken ->
                    ( Model Loading [ "Loading your user" ]
                    , initializeWithUserToken userToken
                    )

                Nothing ->
                    case persistentState.loginToken of
                        Just loginToken ->
                            ( Model (NotLoggedIn (loginSessionInit loginToken)) [ "Checking login status" ]
                            , initializeWithLoginToken loginToken
                            )

                        Nothing ->
                            defaultLoading

        Nothing ->
            defaultLoading



-- UPDATE


type LoginMsg
    = GotLoginUrl (Result Http.Error String)
    | WindowFocus


type UserMsg
    = Flash String


type Msg
    = GotLoginToken (Result Http.Error String)
    | GotUserToken (Result Http.Error String)
    | RegisteredUser (Result Http.Error String)
    | GotMe (Result Http.Error UserSessionJson)
    | GotLoginSession (Result Http.Error LoginSessionJson)
    | GotLoginMsg LoginMsg
    | GotUserMsg UserMsg
    | LogOut


error msg =
    "Error: " ++ msg


success msg =
    "Success: " ++ msg


applyMessages : FlashMessages -> Model -> Model
applyMessages newMessages model =
    { model | messages = newMessages ++ model.messages }


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model.auth ) of
        ( GotLoginToken result, _ ) ->
            case result of
                Ok token ->
                    ( { model | auth = NotLoggedIn (loginSessionInit token) }, Cmd.map GotLoginMsg (getLoginUrl token) )

                Err _ ->
                    ( applyMessages [ error "Failed to retrieve token for logging in" ] model, Cmd.none )

        ( GotUserToken result, _ ) ->
            case result of
                Ok token ->
                    ( { model | auth = Loading }, initializeWithUserToken token )

                Err _ ->
                    ( applyMessages [ error "Failed to retrieve user with login token" ] model, Cmd.none )

        ( RegisteredUser result, NotLoggedIn { loginToken } ) ->
            case result of
                Ok message ->
                    ( model, getUserAccessToken loginToken )

                Err _ ->
                    ( applyMessages [ error "Failed to retrieve registered user with login token" ] model, Cmd.none )

        ( GotLoginSession result, _ ) ->
            case result of
                Ok loginJson ->
                    case loginJson.userId of
                        Just _ ->
                            ( model, getUserAccessToken loginJson.loginToken )

                        Nothing ->
                            case loginJson.displayName of
                                Just _ ->
                                    ( model, registerUser loginJson.loginToken )

                                Nothing ->
                                    ( { model | auth = NotLoggedIn (LoginSession loginJson.loginToken Inactive loginJson.userId) }, Cmd.map GotLoginMsg (getLoginUrl loginJson.loginToken) )

                Err _ ->
                    ( applyMessages [ error "Failed to check login session" ] (Model Loading [])
                    , getLoginAccessToken
                    )

        ( GotMe result, _ ) ->
            case result of
                Ok userJson ->
                    update (GotUserMsg (initUserSession userJson.userToken)) { model | auth = LoggedIn (userSessionInit userJson) }

                Err _ ->
                    ( applyMessages [ error "Failed to log in" ] model, Cmd.none )

        ( GotLoginMsg subMsg, NotLoggedIn loginModel ) ->
            updateLoginSession subMsg loginModel
                |> updateWith NotLoggedIn GotLoginMsg model

        ( GotUserMsg subMsg, LoggedIn userModel ) ->
            updateUserSession subMsg userModel
                |> updateWith LoggedIn GotUserMsg model

        ( LogOut, _ ) ->
            ( Model Loading []
            , getLoginAccessToken
            )

        _ ->
            -- Ignore messages coming from the wrong places
            ( model, Cmd.none )


updateWith : (subModel -> Auth) -> (subMsg -> Msg) -> Model -> ( subModel, FlashMessages, Cmd subMsg ) -> ( Model, Cmd Msg )
updateWith toAuth toMsg model ( subModel, flashMessages, subCmd ) =
    ( { model
        | auth = toAuth subModel
        , messages = flashMessages ++ model.messages
      }
    , Cmd.map toMsg subCmd
    )


updateUserSession : UserMsg -> UserSession -> ( UserSession, FlashMessages, Cmd UserMsg )
updateUserSession msg model =
    case msg of
        Flash message ->
            ( model, [ message ], Cmd.none )


updateLoginSession : LoginMsg -> LoginSession -> ( LoginSession, FlashMessages, Cmd LoginMsg )
updateLoginSession msg model =
    case msg of
        GotLoginUrl result ->
            case result of
                Ok url ->
                    ( { model | loginUrl = Success url }, [], Cmd.none )

                Err _ ->
                    ( { model | loginUrl = Failure }, [ error "Failed to retrieve " ], Cmd.none )

        WindowFocus ->
            ( model, [ error "Don't know how to check login after refocus" ], Cmd.none )


updateLoading : Msg -> LoginSession -> ( LoginSession, FlashMessages, Cmd Msg )
updateLoading msg model =
    case msg of
        _ ->
            ( model, [ "Received unexpected loading message" ], Cmd.none )



-- SUBSCRIPTIONS


port focus : (String -> msg) -> Sub msg


subscriptions : Model -> Sub Msg
subscriptions model =
    case model.auth of
        NotLoggedIn _ ->
            Sub.map GotLoginMsg (focus (always WindowFocus))

        _ ->
            Sub.none



-- VIEW


view : Model -> Html Msg
view model =
    let
        pageView =
            case model.auth of
                Loading ->
                    text "Loading"

                LoggedIn userSession ->
                    Html.div []
                        [ viewUser userSession
                        , Html.button [onClick LogOut] [ text "Log out" ]
                        ]

                NotLoggedIn loginSession ->
                    viewPage loginSession

        flashMessages =
            model.messages
                |> List.map (\s -> Html.p [] [ text s ])
    in
    Html.div []
        [ pageView
        , Html.div [] flashMessages
        ]


viewPage : LoginSession -> Html Msg
viewPage loginSession =
    case loginSession.loginUrl of
        Failure ->
            text "I was unable to load."

        Inactive ->
            Html.div []
                [ text "Loading login session..." ]

        Success loginUrl ->
            a [ href loginUrl ] [ text "Log in with Google" ]


viewUser : UserSession -> Html Msg
viewUser userSession =
    Html.div []
        [ Html.h1 [] [ text userSession.userName ]
        , Html.img [ Html.Attributes.src userSession.userPhotoUrl ] []
        ]



-- HTTP


initializeWithUserToken : String -> Cmd Msg
initializeWithUserToken userToken =
    authRequest
        { method = "GET"
        , body = Http.emptyBody
        , token = userToken
        , path = "/auth/v0/me"
        , expect = Http.expectJson GotMe (meDecoder userToken)
        }


initializeWithLoginToken : String -> Cmd Msg
initializeWithLoginToken loginToken =
    authRequest
        { method = "GET"
        , body = Http.emptyBody
        , token = loginToken
        , path = "/auth/v0/login/session"
        , expect = Http.expectJson GotLoginSession (loginSessionDecoder loginToken)
        }


getLoginAccessToken : Cmd Msg
getLoginAccessToken =
    Http.post
        { body = Http.emptyBody
        , url = authPublicUrl ++ "/auth/v0/login/session"
        , expect = Http.expectJson GotLoginToken tokenDecoder
        }


getUserAccessToken : String -> Cmd Msg
getUserAccessToken loginToken =
    authRequest
        { method = "POST"
        , body = Http.emptyBody
        , path = "/auth/v0/login/session/user"
        , expect = Http.expectJson GotUserToken tokenDecoder
        , token = loginToken
        }


registerUser : String -> Cmd Msg
registerUser loginToken =
    authRequest
        { method = "POST"
        , body = Http.emptyBody
        , path = "/auth/v0/login/session/register"
        , expect = Http.expectJson RegisteredUser successDecoder
        , token = loginToken
        }


getLoginUrl : String -> Cmd LoginMsg
getLoginUrl accessToken =
    authRequest
        { method = "POST"
        , body = Http.emptyBody
        , token = accessToken
        , path = "/auth/v0/google/login_url" ++ "?redirect_uri=" ++ appPublicUrl
        , expect = Http.expectJson GotLoginUrl urlDecoder
        }


initUserSession : String -> UserMsg
initUserSession accessToken =
    Flash "Logged in"


authRequest :
    { method : String
    , path : String
    , token : String
    , body : Http.Body
    , expect : Http.Expect msg
    }
    -> Cmd msg
authRequest req =
    Http.request
        { method = req.method
        , body = req.body
        , headers = [ Http.header "Authorization" ("Bearer " ++ req.token) ]
        , url = authPublicUrl ++ req.path
        , expect = req.expect
        , timeout = Nothing
        , tracker = Nothing
        }


tokenDecoder : Decoder String
tokenDecoder =
    field "access_token" string


successDecoder : Decoder String
successDecoder =
    field "success" string


urlDecoder : Decoder String
urlDecoder =
    field "url" string


meDecoder : String -> Decoder UserSessionJson
meDecoder userToken =
    JD.map3 UserSessionJson (JD.succeed userToken) (JD.at [ "user", "display_name" ] string) (JD.at [ "user", "photo_url" ] string)


loginSessionDecoder : String -> Decoder LoginSessionJson
loginSessionDecoder loginToken =
    JD.map3 LoginSessionJson (JD.succeed loginToken) (JD.maybe (field "user_id" string)) (JD.maybe (JD.at [ "i_am", "display_name" ] string))
